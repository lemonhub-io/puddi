//! Runs `wasi` components with the embedded Wasmtime (WASI 0.3).

use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p3::bindings::Command;
use wasmtime_wasi::{FsPerms, I32Exit, WasiCtx, WasiCtxView, WasiView};

struct State {
    ctx: WasiCtx,
    http: wasmtime_wasi_http::WasiHttpCtx,
    table: ResourceTable,
}

impl wasmtime_wasi_http::WasiHttpView for State {
    fn http(&mut self) -> wasmtime_wasi_http::WasiHttpCtxView<'_> {
        wasmtime_wasi_http::WasiHttpCtxView { hooks: wasmtime_wasi_http::default_hooks(), table: &mut self.table, ctx: &mut self.http }
    }
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
    }
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
        Err(e) => {
            if let Some(exit) = e.downcast_ref::<I32Exit>() {
                return exit.0.clamp(0, 255) as u8;
            }
            if e.downcast_ref::<wasmtime::Trap>().is_some() {
                // the platform layer has already printed panic messages
                return 101;
            }
            eprintln!("error: {e:#}");
            101
        }
    }
}

async fn run_async(bytes: &[u8], program: &str, args: &[String]) -> wasmtime::Result<u8> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.wasm_component_model_more_async_builtins(true);
    config.wasm_component_model_async_stackful(true);
    config.wasm_component_model_threading(true);
    config.wasm_exceptions(true);
    config.wasm_gc(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::<State>::new(&engine);
    wasmtime_wasi::p3::add_to_linker(&mut linker)?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)?;
    let component = Component::new(&engine, bytes)?;
    let mut builder = WasiCtx::builder();
    let mut argv = vec![program.to_string()];
    argv.extend(args.iter().cloned());
    builder.inherit_stdio().inherit_env().args(&argv);
    builder.preopened_dir(".", ".", FsPerms::ReadWrite)?;
    builder.preopened_dir("/", "/", FsPerms::ReadWrite)?;
    let mut store = Store::new(&engine, State { ctx: builder.build(), http: wasmtime_wasi_http::WasiHttpCtx::new(), table: ResourceTable::default() });
    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    let result = store.run_concurrent(async move |store| command.wasi_cli_run().call_run(store).await).await??;
    Ok(if result.is_ok() { 0 } else { 1 })
}
