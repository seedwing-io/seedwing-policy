use clap::Parser;
use std::path::PathBuf;
use std::str;
use wasmtime::{
    component::{bindgen, Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::preview2::wasi::command::add_to_linker;
use wasmtime_wasi::preview2::{Table, WasiCtx, WasiCtxBuilder, WasiView};

bindgen!({
    path: "../../wit",
    world: "static-evaluator",
    async: true
});

struct CommandCtx {
    table: Table,
    wasi_ctx: WasiCtx,
}

impl WasiView for CommandCtx {
    fn table(&self) -> &Table {
        &self.table
    }
    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }
    fn ctx(&self) -> &WasiCtx {
        &self.wasi_ctx
    }
    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

#[derive(clap::Parser, Debug)]
#[command(
    author,
    about = "Wasm runner",
    long_about = None
)]
pub struct Args {
    #[arg(short = 'm', long = "module", value_name = "FILE")]
    pub(crate) module: PathBuf,

    #[arg(short = 'i', long = "input", value_name = "String")]
    pub(crate) input: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> wasmtime::Result<()> {
    let args = Args::parse();

    let path = &args.module;
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    let component = Component::from_file(&engine, path)?;

    let args: Vec<_> = std::env::args().collect();
    let vars: Vec<_> = std::env::vars().collect();
    let mut table = Table::new();
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .set_args(&args)
        .set_env(&vars)
        .build(&mut table)?;
    let ctx = CommandCtx { table, wasi_ctx };

    let mut store = Store::new(&engine, ctx);
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker)?;

    let (reactor, _instance) =
        StaticEvaluator::instantiate_async(&mut store, &component, &linker).await?;
    let string: String = reactor.call_run(&mut store).await?;
    println!("{string:?}");
    Ok(())
}
