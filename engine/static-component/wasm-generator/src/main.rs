use clap::Parser;
use static_generator::{self, BuildType, GenConfig};
use std::fs;
use std::path::PathBuf;

#[derive(clap::Parser, Debug)]
#[command(
    author,
    about = "Seedwing Policy Component Generator",
    long_about = None
)]
pub struct Args {
    #[arg(short = 'p', long = "policy", value_name = "FILE")]
    pub(crate) policy: PathBuf,

    #[arg(short = 'n', long = "name", value_name = "String")]
    pub(crate) policy_name: String,

    #[arg(
        short = 'm',
        long = "modules",
        value_name = "DIR",
        default_value = "modules"
    )]
    pub(crate) modules_dir: PathBuf,

    #[arg(
        short = 'o',
        long = "outputdir",
        value_name = "DIR",
        default_value = "working/target"
    )]
    pub(crate) output_dir: PathBuf,
}

fn main() {
    let args = Args::parse();
    let policy = fs::read_to_string(&args.policy).unwrap();
    let policy_name = &args.policy_name;
    let config = GenConfig {
        policy: policy.to_string(),
        policy_name: policy_name.to_string(),
        build_type: BuildType::Debug,
        modules_dir: args.modules_dir,
        output_dir: args.output_dir,
    };
    match static_generator::generate(&config) {
        Ok(composed_path) => println!("Composed into webassembly component:\n{composed_path:?}"),
        Err(e) => eprintln!("Error while generating: {}", e),
    }
}
