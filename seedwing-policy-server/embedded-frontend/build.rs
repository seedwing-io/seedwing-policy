use static_files::resource_dir;
use std::env;
use std::path::Path;
use std::process::Command;

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    // console assets

    println!("cargo:rerun-if-changed=../../seedwing-policy-frontend/Cargo.toml");
    println!("cargo:rerun-if-changed=../../seedwing-policy-frontend/Cargo.lock");
    println!("cargo:rerun-if-changed=../../seedwing-policy-frontend/Trunk.toml");
    println!("cargo:rerun-if-changed=../../seedwing-policy-frontend/package.json");
    println!("cargo:rerun-if-changed=../../seedwing-policy-frontend/yarn.lock");
    println!("cargo:rerun-if-changed=../../seedwing-policy-frontend/src");
    println!("cargo:rerun-if-changed=../../seedwing-policy-frontend/assets");

    let output = Command::new("trunk")
        .args([
            "build",
            "--release",
            "-d",
            "../seedwing-policy-server/embedded-frontend/dist",
            "--public-url",
            "/console",
        ])
        .current_dir("../../seedwing-policy-frontend")
        .output()
        .expect("failed to execute frontend build");

    if !output.status.success() {
        panic!(
            "Failed to run 'trunk':\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let mut assets = resource_dir("dist");
    assets
        .with_generated_filename(
            Path::new(&env::var("OUT_DIR").unwrap()).join("generated-console.rs"),
        )
        .with_generated_fn("generate_console_assets");

    assets.build()?;

    // done

    Ok(())
}
