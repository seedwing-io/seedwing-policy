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
    .with_generated_fn("generate_docs")
    .with_module_name("generated_docs");

    docs.build()?;

    let mut assets = resource_dir("./src/assets");
    assets
        .with_generated_filename(
            Path::new(&env::var("OUT_DIR").unwrap()).join("generated-assets.rs"),
        )
        .with_generated_fn("generate_assets")
        .with_module_name("generated_assets");

    assets.build()?;

    NpmBuild::new("./web")
        .install()?
        .change_detection()
        .to_resource_dir()
        .build()?;

    Ok(())
}
