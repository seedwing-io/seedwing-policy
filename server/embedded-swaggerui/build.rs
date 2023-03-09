use static_files::resource_dir;
use std::env;
use std::path::Path;

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=dist/");

    let out_dir = env::var_os("OUT_DIR").unwrap();

    let mut assets = resource_dir("./dist");
    assets
        .with_generated_filename(Path::new(&out_dir).join("generated.rs"))
        .with_generated_fn("generate_assets");

    assets.build()?;

    // done

    Ok(())
}
