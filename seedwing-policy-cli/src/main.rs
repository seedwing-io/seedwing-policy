use clap::Arg;
use clap::Parser;
use crate::cli::{Cli, Command};

mod cli;

#[tokio::main]
async fn main() -> Result<(), ()>{
    let cli: Cli = Cli::parse();
    cli.run().await
}