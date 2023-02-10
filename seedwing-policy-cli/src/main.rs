use crate::cli::{Cli, Command};
use clap::Arg;
use clap::Parser;

mod cli;
mod eval;
mod explain;
mod verify;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let cli: Cli = Cli::parse();
    cli.run().await
}
