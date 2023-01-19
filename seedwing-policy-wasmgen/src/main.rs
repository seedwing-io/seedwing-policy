use seedwing_policy_engine::{lang::builder::Builder, runtime::sources::Ephemeral, *};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use wasm_encoder::*;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    if args.len() < 3 {
        eprintln!("usage: {} <policy.dog> <output.wasm>", program);
        return;
    }
    let output = args[2].clone();

    let src = Ephemeral::new(
        "foo",
        r#"
        pattern allow = true
        "#,
    );

    let mut builder = Builder::new();
    let result = builder.build(src.iter());
    println!("RESULT: {:#?}", result);
    let runtime = builder.finish().await.unwrap();
    let module = runtime.emit("foo::allow").unwrap();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(output)
        .unwrap();

    file.write_all(&module.finish());
    println!("Done!");
}
