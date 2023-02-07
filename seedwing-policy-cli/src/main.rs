use clap::Arg;
use clap::Parser;
use crate::cli::{Cli, Command};

mod cli;
mod verify;
mod eval;

#[tokio::main]
async fn main() -> Result<(), ()>{
    let cli: Cli = Cli::parse();
    cli.run().await
}