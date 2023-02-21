use static_files::{resource_dir, NpmBuild};
use std::env;
use std::path::Path;

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    // documentation

    println!("cargo:rerun-if-changed=ROOT/");

    let mut docs = resource_dir("../dogma");
    docs.with_generated_filename(
        Path::new(&env::var("OUT_DIR").unwrap()).join("generated-ROOT.rs"),
    )
    .with_generated_fn("generate_docs");

    docs.build()?;

    // examples

    println!("cargo:rerun-if-changed=examples/");

    let mut examples = resource_dir("../examples");
    examples
        .with_generated_filename(
            Path::new(&env::var("OUT_DIR").unwrap()).join("generated-examples.rs"),
        )
        .with_generated_fn("generate_examples");

    examples.build()?;

    // static web assets

    let mut assets = resource_dir("./src/assets");
    assets
        .with_generated_filename(
            Path::new(&env::var("OUT_DIR").unwrap()).join("generated-assets.rs"),
        )
        .with_generated_fn("generate_assets");

    assets.build()?;

    // npm assets

    println!("cargo:rerun-if-changed=web/copy-assets.sh");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/yarn.lock");

    let mut npm_assets = NpmBuild::new("./web")
        .target("./web/dist")
        .executable("yarn")
        .install()?
        .run("build")?
        .to_resource_dir();
    npm_assets
        .with_generated_filename(
            Path::new(&env::var("OUT_DIR").unwrap()).join("generated-npm-assets.rs"),
        )
        .with_generated_fn("generate_npm_assets");

    npm_assets.build()?;

    // done

    Ok(())
}
