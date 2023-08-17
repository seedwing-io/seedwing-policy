use crate::seedwing::policy::types;
use std::collections::HashMap;
use wasmtime::{
    component::{bindgen, Component, Linker},
    Config, Engine as WasmtimeEngine, Store,
};
use wasmtime_wasi::preview2::wasi::command::add_to_linker;
use wasmtime_wasi::preview2::{Table, WasiCtx, WasiCtxBuilder, WasiView};

bindgen!({
    path: "../../wit",
    world: "engine-world",
    async: true,
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> wasmtime::Result<()> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = WasmtimeEngine::new(&config)?;
    let bytes = include_bytes!("../../../../target/seedwing-policy-engine-component.wasm");
    let component = Component::from_binary(&engine, bytes)?;

    let mut table = Table::new();
    let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build(&mut table)?;
    let ctx = CommandCtx { table, wasi_ctx };

    let mut store = Store::new(&engine, ctx);
    let mut linker = Linker::new(&engine);
    add_to_linker(&mut linker)?;

    let (wit, _instance) = EngineWorld::instantiate_async(&mut store, &component, &linker).await?;
    let engine = wit.interface0;

    println!(
        "Seedwing Policy Engine version: {}\n",
        engine.call_version(&mut store).await?
    );

    let policies = [];
    let data = [];
    let policy = "pattern dog = { name: string, trained: boolean }";
    let policy_name = "dog";

    let obj_name = types::Object {
        key: "name".to_string(),
        value: types::ObjectValue::String("goodboy".to_string()),
    };
    let obj_trained = types::Object {
        key: "trained".to_string(),
        value: types::ObjectValue::Boolean(true),
    };
    let input = types::RuntimeValue::Object(vec![obj_name, obj_trained]);

    let result = engine
        .call_eval(&mut store, &policies, &data, policy, policy_name, &input)
        .await?;

    match result {
        Ok(context) => {
            let evaluation_result = context.evaluation_result;
            println!("EvaluationResult:");
            let input: types::RuntimeValue = evaluation_result.input;
            println!("input: {:#?}\n", input);

            let ty: types::Pattern = evaluation_result.ty;
            println!("ty: {:#?}\n", ty);

            let rationale: types::Rationale = evaluation_result.rationale;
            println!("rationale: {:#?}\n", rationale);

            let output: String = evaluation_result.output;
            println!("output: {:#?}\n", output);

            let pattern_map: HashMap<String, types::Pattern> =
                context.pattern_map.into_iter().collect();
            println!("pattern_map: {:#?}\n", pattern_map.keys());

            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Policy Engine error {:?}", e)),
    }
}
