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
    let rt = match runtime() {
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

type Warm = (wasmtime::Result<Engine>, std::io::Result<tokio::runtime::Runtime>);
static PREWARM: std::sync::Mutex<Option<std::thread::JoinHandle<Warm>>> = std::sync::Mutex::new(None);
static WARM_ENGINE: std::sync::Mutex<Option<wasmtime::Result<Engine>>> = std::sync::Mutex::new(None);

/// Starts creating the engine and the async runtime on another thread, so that it
/// overlaps compiling the program.
pub fn prewarm() {
    if let Ok(mut slot) = PREWARM.lock() {
        *slot = Some(std::thread::spawn(|| (new_engine(), new_runtime())));
    }
}

fn new_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()
}

/// The prewarmed runtime (keeping the prewarmed engine for `engine`), or a new one.
fn runtime() -> std::io::Result<tokio::runtime::Runtime> {
    if let Some(h) = PREWARM.lock().ok().and_then(|mut slot| slot.take()) {
        if let Ok((engine, rt)) = h.join() {
            if let Ok(mut e) = WARM_ENGINE.lock() {
                *e = Some(engine);
            }
            return rt;
        }
    }
    new_runtime()
}

fn engine() -> wasmtime::Result<Engine> {
    if let Some(e) = WARM_ENGINE.lock().ok().and_then(|mut e| e.take()) {
        return e;
    }
    new_engine()
}

fn new_engine() -> wasmtime::Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.wasm_component_model_more_async_builtins(true);
    config.wasm_component_model_async_stackful(true);
    config.wasm_component_model_threading(true);
    config.wasm_exceptions(true);
    config.wasm_gc(true);
    config.wasm_function_references(true);
    // Puddi code allocates heavily and reads GC references in hot loops: the copying
    // collector bump-allocates and needs no reference-count barriers, unlike DRC.
    // PUD_WASMTIME_GC and PUD_WASMTIME_INLINE override the choices for measurements.
    config.collector(match std::env::var("PUD_WASMTIME_GC").as_deref() {
        Ok("drc") => wasmtime::Collector::DeferredReferenceCounting,
        _ => wasmtime::Collector::Copying,
    });
    // a 16 MiB starting heap avoids most early collections and regrowths (sorting a million
    // numbers runs 1.7x faster) for a few MiB more memory; PUD_WASMTIME_HEAP_MB overrides it
    let heap_mb = std::env::var("PUD_WASMTIME_HEAP_MB").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(16);
    config.gc_heap_initial_size(heap_mb << 20);
    // small runtime helpers (List indexing, overflow checks, iterator adapters) are
    // separate wasm functions; Cranelift only removes their call overhead when inlining
    if std::env::var("PUD_WASMTIME_INLINE").as_deref() != Ok("0") {
        config.compiler_inlining(wasmtime::Inlining::Yes);
    }
    Engine::new(&config)
}

/// Compiles `bytes`, or loads the machine code compiled by an earlier run of the same
/// component from the user's cache directory (`PUD_NO_CACHE=1` turns this off).
fn load_component(engine: &Engine, bytes: &[u8]) -> wasmtime::Result<Component> {
    let Some(dir) = cache_dir() else { return Component::new(engine, bytes) };
    use std::hash::{Hash, Hasher};
    let mut key = String::new();
    for salt in [0u8, 1] {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (salt, env!("CARGO_PKG_VERSION"), std::env::var("PUD_WASMTIME_GC").ok(), std::env::var("PUD_WASMTIME_INLINE").ok(), std::env::var("PUD_WASMTIME_HEAP_MB").ok(), bytes).hash(&mut h);
        key.push_str(&format!("{:016x}", h.finish()));
    }
    let path = dir.join(format!("{key}.cwasm"));
    if path.exists() {
        // SAFETY: the file was written by `precompile_component` below into a directory only
        // this user can write; Wasmtime rejects code compiled for another engine configuration.
        if let Ok(c) = unsafe { Component::deserialize_file(engine, &path) } {
            return Ok(c);
        }
    }
    let code = engine.precompile_component(bytes)?;
    let _ = std::fs::create_dir_all(&dir);
    let tmp = dir.join(format!("{key}.{}.tmp", std::process::id()));
    if std::fs::write(&tmp, &code).is_ok() && std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    // SAFETY: `code` was just produced by this engine
    unsafe { Component::deserialize(engine, &code) }
}

fn cache_dir() -> Option<std::path::PathBuf> {
    if std::env::var_os("PUD_NO_CACHE").is_some() {
        return None;
    }
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Library/Caches"))
    } else {
        std::env::var_os("XDG_CACHE_HOME").map(std::path::PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
    };
    base.map(|b| b.join("pud").join("wasmtime"))
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
    use crate::driver::phase;
    let engine = engine()?;
    phase("engine");
    let mut linker = Linker::<State>::new(&engine);
    wasmtime_wasi::p3::add_to_linker(&mut linker)?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)?;
    add_workers(&mut linker)?;
    phase("linker");
    let component = load_component(&engine, bytes)?;
    phase("load component");
    let mut argv = vec![program.to_string()];
    argv.extend(args.iter().cloned());
    let shared = Arc::new(Shared { engine: engine.clone(), component: component.clone(), linker: linker.clone(), argv: argv.clone() });
    let mut store = Store::new(&engine, new_state(&argv, Some(shared))?);
    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    phase("instantiate");
    let result = store.run_concurrent(async move |store| command.wasi_cli_run().call_run(store).await).await??;
    Ok(if result.is_ok() { 0 } else { 1 })
}

/// Host side of `wasi:logging/logging`: messages go to stderr.
fn add_logging(linker: &mut Linker<State>) -> wasmtime::Result<()> {
    linker.instance("wasi:logging/logging")?.func_new("log", |_store, _ty, params, _results| {
        let level = match &params[0] {
            wasmtime::component::Val::Enum(l) => l.clone(),
            _ => "info".into(),
        };
        let msg = match &params[2] {
            wasmtime::component::Val::String(s) => s.clone(),
            _ => String::new(),
        };
        if level == "info" {
            println!("{msg}");
        } else {
            eprintln!("[{level}] {msg}");
        }
        Ok(())
    })?;
    Ok(())
}

fn parse_arg(ty: &wasmtime::component::Type, s: &str) -> wasmtime::Result<wasmtime::component::Val> {
    use wasmtime::component::{Type, Val};
    let bad = || wasmtime::format_err!("cannot read `{s}` as an argument of type {ty:?}");
    Ok(match ty {
        Type::Bool => Val::Bool(s.parse().map_err(|_| bad())?),
        Type::S8 => Val::S8(s.parse().map_err(|_| bad())?),
        Type::U8 => Val::U8(s.parse().map_err(|_| bad())?),
        Type::S16 => Val::S16(s.parse().map_err(|_| bad())?),
        Type::U16 => Val::U16(s.parse().map_err(|_| bad())?),
        Type::S32 => Val::S32(s.parse().map_err(|_| bad())?),
        Type::U32 => Val::U32(s.parse().map_err(|_| bad())?),
        Type::S64 => Val::S64(s.parse().map_err(|_| bad())?),
        Type::U64 => Val::U64(s.parse().map_err(|_| bad())?),
        Type::Float32 => Val::Float32(s.parse().map_err(|_| bad())?),
        Type::Float64 => Val::Float64(s.parse().map_err(|_| bad())?),
        Type::Char => Val::Char(s.chars().next().ok_or_else(bad)?),
        Type::String => Val::String(s.to_string()),
        Type::Enum(_) => Val::Enum(s.to_string()),
        _ => return Err(bad()),
    })
}

fn show_val(v: &wasmtime::component::Val) -> String {
    use wasmtime::component::Val;
    let list = |xs: &[Val]| xs.iter().map(show_val).collect::<Vec<_>>().join(", ");
    match v {
        Val::Bool(b) => b.to_string(),
        Val::S8(x) => x.to_string(),
        Val::U8(x) => x.to_string(),
        Val::S16(x) => x.to_string(),
        Val::U16(x) => x.to_string(),
        Val::S32(x) => x.to_string(),
        Val::U32(x) => x.to_string(),
        Val::S64(x) => x.to_string(),
        Val::U64(x) => x.to_string(),
        Val::Float32(x) => x.to_string(),
        Val::Float64(x) => x.to_string(),
        Val::Char(c) => format!("{c:?}"),
        Val::String(s) => format!("{s:?}"),
        Val::List(xs) | Val::FixedLengthList(xs) => format!("[{}]", list(xs)),
        Val::Tuple(xs) => format!("({})", list(xs)),
        Val::Record(fs) => format!("{{{}}}", fs.iter().map(|(k, v)| format!("{k}: {}", show_val(v))).collect::<Vec<_>>().join(", ")),
        Val::Variant(n, p) => match p {
            Some(p) => format!("{n}({})", show_val(p)),
            None => n.clone(),
        },
        Val::Enum(n) => n.clone(),
        Val::Option(o) => match o {
            Some(x) => format!("some({})", show_val(x)),
            None => "none".into(),
        },
        Val::Result(r) => match r {
            Ok(Some(x)) => format!("ok({})", show_val(x)),
            Ok(None) => "ok".into(),
            Err(Some(x)) => format!("err({})", show_val(x)),
            Err(None) => "err".into(),
        },
        Val::Flags(fs) => format!("{{{}}}", fs.join(", ")),
        other => format!("{other:?}"),
    }
}

/// Calls export `name` of a plugin component with arguments parsed from text
/// and prints the result. Host imports that the runtime does not provide trap.
pub fn invoke(component: &[u8], program: &str, name: &str, args: &[String]) -> u8 {
    let rt = match runtime() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: cannot start the async runtime: {e}");
            return 1;
        }
    };
    match rt.block_on(invoke_async(component, program, name, args)) {
        Ok(code) => code,
        Err(e) => exit_code(&e),
    }
}

async fn invoke_async(bytes: &[u8], program: &str, name: &str, args: &[String]) -> wasmtime::Result<u8> {
    let engine = engine()?;
    let component = load_component(&engine, bytes)?;
    let mut linker = Linker::<State>::new(&engine);
    wasmtime_wasi::p3::add_to_linker(&mut linker)?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)?;
    add_workers(&mut linker)?;
    add_logging(&mut linker)?;
    stub_host_imports(&mut linker, &component, &engine)?;
    let argv = vec![program.to_string()];
    let shared = Arc::new(Shared { engine: engine.clone(), component: component.clone(), linker: linker.clone(), argv: argv.clone() });
    let mut store = Store::new(&engine, new_state(&argv, Some(shared))?);
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let idx = match name.split_once('#') {
        Some((iface, f)) => {
            let i = instance.get_export_index(&mut store, None, iface).ok_or_else(|| wasmtime::format_err!("the plugin does not export `{iface}`"))?;
            instance.get_export_index(&mut store, Some(&i), f)
        }
        None => instance.get_export_index(&mut store, None, name),
    }
    .ok_or_else(|| wasmtime::format_err!("the plugin does not export a function `{name}`"))?;
    let func = instance.get_func(&mut store, &idx).ok_or_else(|| wasmtime::format_err!("`{name}` is not a function"))?;
    let ty = func.ty(&store);
    let ptypes: Vec<_> = ty.params().map(|(_, t)| t).collect();
    if ptypes.len() != args.len() {
        return Err(wasmtime::format_err!("`{name}` takes {} argument(s), got {}", ptypes.len(), args.len()));
    }
    let params = ptypes.iter().zip(args).map(|(t, a)| parse_arg(t, a)).collect::<wasmtime::Result<Vec<_>>>()?;
    let mut results = vec![wasmtime::component::Val::Bool(false); ty.results().len()];
    let results = store
        .run_concurrent(async move |a| -> wasmtime::Result<Vec<wasmtime::component::Val>> {
            func.call_concurrent(a, &params, &mut results).await?;
            Ok(results)
        })
        .await??;
    for r in &results {
        println!("{}", show_val(r));
    }
    Ok(0)
}

/// A host resource type for stubbed imports.
struct StubResource;

/// Defines the plugin's application-specific imports (anything outside
/// `wasi:` and `puddi:`) as functions that trap when called. Wasmtime's
/// `define_unknown_imports_as_traps` would also shadow the WASI 0.3.0 host
/// functions that satisfy 0.3.2 imports through semver matching.
fn stub_host_imports(linker: &mut Linker<State>, component: &Component, engine: &Engine) -> wasmtime::Result<()> {
    use wasmtime::component::types::ComponentItem;
    fn stub(li: &mut wasmtime::component::LinkerInstance<'_, State>, parent: &str, name: &str, item: ComponentItem) -> wasmtime::Result<()> {
        let full = if parent.is_empty() { name.to_string() } else { format!("{parent}#{name}") };
        match item {
            ComponentItem::ComponentFunc(f) if f.async_() => li.func_new_concurrent(name, move |_, _, _, _| {
                let full = full.clone();
                Box::pin(async move { Err(wasmtime::format_err!("the host function `{full}` is not available in `pud run`")) })
            }),
            ComponentItem::ComponentFunc(_) => li.func_new(name, move |_, _, _, _| Err(wasmtime::format_err!("the host function `{full}` is not available in `pud run`"))),
            ComponentItem::Resource(_) => li.resource(name, wasmtime::component::ResourceType::host::<StubResource>(), |_, _| Ok(())),
            _ => Ok(()),
        }
    }
    let ty = component.component_type();
    for (name, ext) in ty.imports(engine) {
        if name.starts_with("wasi:") || name.starts_with("puddi:") {
            continue;
        }
        match ext.ty {
            ComponentItem::ComponentInstance(inst) => {
                let mut li = linker.instance(name)?;
                for (fname, fe) in inst.exports(engine) {
                    stub(&mut li, name, fname, fe.ty)?;
                }
            }
            other => stub(&mut linker.root(), "", name, other)?,
        }
    }
    Ok(())
}
