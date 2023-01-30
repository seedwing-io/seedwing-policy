use static_files::{resource_dir, NpmBuild};
use std::env;
use std::path::Path;

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=docs/");

    let mut docs = resource_dir("../docs");
    docs.with_generated_filename(
        Path::new(&env::var("OUT_DIR").unwrap()).join("generated-docs.rs"),
    )
    .with_generated_fn("generate_docs");

    docs.build()?;

    let mut assets = resource_dir("./src/assets");
    assets
        .with_generated_filename(
            Path::new(&env::var("OUT_DIR").unwrap()).join("generated-assets.rs"),
        )
        .with_generated_fn("generate_assets");

    assets.build()?;

    let mut npm_assets = NpmBuild::new("./web")
        .install()?
        .run("build")?
        .target("./web/dist")
        .change_detection()
        .to_resource_dir();
    npm_assets
        .with_generated_filename(
            Path::new(&env::var("OUT_DIR").unwrap()).join("generated-npm-assets.rs"),
        )
        .with_generated_fn("generate_npm_assets");

    npm_assets.build()?;

    Ok(())
}
