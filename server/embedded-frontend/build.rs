use static_files::resource_dir;
use std::env;
use std::path::Path;
use std::process::Command;

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    // console assets

    println!("cargo:rerun-if-changed=../../frontend/Cargo.toml");
    println!("cargo:rerun-if-changed=../../frontend/Cargo.lock");
    println!("cargo:rerun-if-changed=../../frontend/Trunk.toml");
    println!("cargo:rerun-if-changed=../../frontend/package.json");
    println!("cargo:rerun-if-changed=../../frontend/src");
    println!("cargo:rerun-if-changed=../../frontend/assets");

    let npm = Command::new("npm")
        .args(["ci"])
        .current_dir("../../frontend")
        .output()
        .expect("failed to execute frontend build");

    if !npm.status.success() {
        panic!(
            "Failed to run 'npm':\n{}\n{}",
            String::from_utf8_lossy(&npm.stdout),
            String::from_utf8_lossy(&npm.stderr)
        );
    }

    let trunk = Command::new("trunk")
        .args([
            "build",
            "--release",
            "-d",
            "../server/embedded-frontend/dist",
            "--public-url",
            "/",
        ])
        .current_dir("../../frontend")
        .output()
        .expect("failed to execute frontend build");

    if !trunk.status.success() {
        panic!(
            "Failed to run 'trunk':\n{}\n{}",
            String::from_utf8_lossy(&trunk.stdout),
            String::from_utf8_lossy(&trunk.stderr)
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
