//! Runs `wasi` components with the embedded Wasmtime (WASI 0.3), and hosts
//! the `puddi:runtime` worker interfaces: each worker is a separate instance
//! of the same component running on its own OS thread.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use wasmtime::component::{Accessor, Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p3::bindings::Command;
use wasmtime_wasi::{FsPerms, I32Exit, WasiCtx, WasiCtxView, WasiView};

type Inbox = Arc<Mutex<UnboundedReceiver<String>>>;

/// What a store needs to start workers.
struct Shared {
    engine: Engine,
    component: Component,
    linker: Linker<State>,
    argv: Vec<String>,
}

struct State {
    ctx: WasiCtx,
    http: wasmtime_wasi_http::WasiHttpCtx,
    table: ResourceTable,
    shared: Option<Arc<Shared>>,
    workers: HashMap<u32, (UnboundedSender<String>, Inbox)>,
    next_worker: u32,
    /// The worker side of the channel (in worker instances).
    port_in: Option<Inbox>,
    port_out: Option<UnboundedSender<String>>,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
    }
}

impl wasmtime_wasi_http::WasiHttpView for State {
    fn http(&mut self) -> wasmtime_wasi_http::WasiHttpCtxView<'_> {
        wasmtime_wasi_http::WasiHttpCtxView { hooks: wasmtime_wasi_http::default_hooks(), table: &mut self.table, ctx: &mut self.http }
    }
}

fn new_state(argv: &[String], shared: Option<Arc<Shared>>) -> wasmtime::Result<State> {
    let mut builder = WasiCtx::builder();
    builder.inherit_stdio().inherit_env().args(argv);
    builder.preopened_dir(".", ".", FsPerms::ReadWrite)?;
    builder.preopened_dir("/", "/", FsPerms::ReadWrite)?;
    Ok(State {
        ctx: builder.build(),
        http: wasmtime_wasi_http::WasiHttpCtx::new(),
        table: ResourceTable::default(),
        shared,
        workers: HashMap::new(),
        next_worker: 1,
        port_in: None,
        port_out: None,
    })
}

/// Maps an error from the guest to an exit code.
fn exit_code(e: &wasmtime::Error) -> u8 {
    if let Some(exit) = e.downcast_ref::<I32Exit>() {
        return exit.0.clamp(0, 255) as u8;
    }
    if e.downcast_ref::<wasmtime::Trap>().is_none() {
        // traps from panics were already reported by the platform layer
        eprintln!("error: {e:#}");
    }
    101
}

/// Runs the component and returns the process exit code.
pub fn run(component: &[u8], program: &str, args: &[String]) -> u8 {
    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: cannot start the async runtime: {e}");
            return 1;
        }
    };
    match rt.block_on(run_async(component, program, args)) {
        Ok(code) => code,
        Err(e) => exit_code(&e),
    }
}

fn engine() -> wasmtime::Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.wasm_component_model_more_async_builtins(true);
    config.wasm_component_model_async_stackful(true);
    config.wasm_component_model_threading(true);
    config.wasm_exceptions(true);
    config.wasm_gc(true);
    config.wasm_function_references(true);
    Engine::new(&config)
}

fn add_workers(linker: &mut Linker<State>) -> wasmtime::Result<()> {
    let mut w = linker.instance("puddi:runtime/workers@0.1.0")?;
    w.func_wrap("start", |mut store, (entry,): (u32,)| {
        let st = store.data_mut();
        let shared = st.shared.clone().ok_or_else(|| wasmtime::format_err!("workers are not available here"))?;
        let id = st.next_worker;
        st.next_worker += 1;
        let (to_worker, worker_in) = unbounded_channel();
        let (worker_out, from_worker) = unbounded_channel();
        st.workers.insert(id, (to_worker, Arc::new(Mutex::new(from_worker))));
        spawn_worker(shared, entry, worker_in, worker_out);
        Ok((id,))
    })?;
    w.func_wrap("send", |mut store, (id, msg): (u32, String)| {
        if let Some((tx, _)) = store.data_mut().workers.get(&id) {
            let _ = tx.send(msg);
        }
        Ok(())
    })?;
    w.func_wrap_concurrent("recv", |accessor: &Accessor<State>, (id,): (u32,)| {
        Box::pin(async move {
            let inbox = accessor.with(|mut a| a.data_mut().workers.get(&id).map(|w| w.1.clone()));
            Ok((match inbox {
                Some(rx) => rx.lock().await.recv().await,
                None => None,
            },))
        })
    })?;
    w.func_wrap("close", |mut store, (id,): (u32,)| {
        store.data_mut().workers.remove(&id);
        Ok(())
    })?;
    let mut p = linker.instance("puddi:runtime/port@0.1.0")?;
    p.func_wrap("send", |mut store, (msg,): (String,)| {
        if let Some(tx) = &store.data_mut().port_out {
            let _ = tx.send(msg);
        }
        Ok(())
    })?;
    p.func_wrap_concurrent("recv", |accessor: &Accessor<State>, (): ()| {
        Box::pin(async move {
            let inbox = accessor.with(|mut a| a.data_mut().port_in.clone());
            Ok((match inbox {
                Some(rx) => rx.lock().await.recv().await,
                None => None,
            },))
        })
    })?;
    p.func_wrap("done", |mut store, (): ()| {
        store.data_mut().port_out = None;
        Ok(())
    })?;
    Ok(())
}

/// Starts worker entry `entry` in a new instance on its own thread.
fn spawn_worker(shared: Arc<Shared>, entry: u32, inbox: UnboundedReceiver<String>, outbox: UnboundedSender<String>) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("error: cannot start a worker: {e}");
                std::process::exit(101);
            }
        };
        let res: wasmtime::Result<()> = rt.block_on(async {
            let mut state = new_state(&shared.argv, Some(shared.clone()))?;
            state.port_in = Some(Arc::new(Mutex::new(inbox)));
            state.port_out = Some(outbox);
            let mut store = Store::new(&shared.engine, state);
            let instance = shared.linker.instantiate_async(&mut store, &shared.component).await?;
            let iface = instance.get_export_index(&mut store, None, "puddi:runtime/worker-main@0.1.0").ok_or_else(|| wasmtime::format_err!("the component does not export puddi:runtime/worker-main"))?;
            let start = instance.get_export_index(&mut store, Some(&iface), "start").ok_or_else(|| wasmtime::format_err!("worker-main has no start function"))?;
            let start = instance.get_typed_func::<(u32,), ()>(&mut store, &start)?;
            store.run_concurrent(async move |a| start.call_concurrent(a, (entry,)).await).await??;
            Ok(())
        });
        if let Err(e) = res {
            // a failing worker ends the program, as on the JavaScript host
            std::process::exit(exit_code(&e) as i32);
        }
    });
}

async fn run_async(bytes: &[u8], program: &str, args: &[String]) -> wasmtime::Result<u8> {
    let engine = engine()?;
    let mut linker = Linker::<State>::new(&engine);
    wasmtime_wasi::p3::add_to_linker(&mut linker)?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)?;
    add_workers(&mut linker)?;
    let component = Component::new(&engine, bytes)?;
    let mut argv = vec![program.to_string()];
    argv.extend(args.iter().cloned());
    let shared = Arc::new(Shared { engine: engine.clone(), component: component.clone(), linker: linker.clone(), argv: argv.clone() });
    let mut store = Store::new(&engine, new_state(&argv, Some(shared))?);
    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    let result = store.run_concurrent(async move |store| command.wasi_cli_run().call_run(store).await).await??;
    Ok(if result.is_ok() { 0 } else { 1 })
}
