use std::env;

use std::process::Command;
use tempfile::TempDir;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        let input = args[1].clone();
        let output = args[2].clone();

        let dir: TempDir = tempfile::tempdir().unwrap();
        println!("Created temp dir {}", dir.path().display());
        std::fs::write(dir.path().join("Cargo.toml"), CARGO_TEMPLATE).unwrap();

        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), CARGO_TEMPLATE).unwrap();

        std::fs::copy(input, src.join("policy.dog")).unwrap();
        std::fs::write(src.join("main.rs"), MAIN_TEMPLATE).unwrap();

        Command::new("cargo")
            .current_dir(dir.path())
            .arg("build")
            .arg("--release")
            .arg("--target")
            .arg("wasm32-wasi")
            .output()
            .expect("failed compiling policy");
        std::fs::copy(
            dir.path()
                .join("target")
                .join("wasm32-wasi")
                .join("release")
                .join("generated-policy.wasm"),
            &output,
        )
        .unwrap();
        println!("Policy compiled into {}", output);
    } else {
        println!("usage: {} <policy.dog> <output.wasm>", args[0]);
    }
}

static CARGO_TEMPLATE: &str = r#"
[package]
name = "generated-policy"
version = "0.1.0"
edition = "2021"

[dependencies]
seedwing-policy-wasm = { version = "0.1.0", path = "/home/lulf/dev/trustification/seedwing-policy/seedwing-policy-wasm", default-features = false }
wasi = "0.11"
"#;

static MAIN_TEMPLATE: &str = r#"
use seedwing_policy_wasm::*;
use std::env;

static POLICY: &str = include_str!("policy.dog");

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    if args.len() < 3 {
        eprintln!("usage: {} <path> <type>:<value>", program);
        return;
    }

    run(POLICY, &args[1], &args[2]);
}
"#;
