use crate::cli::Cli;

use clap::Parser;

mod cli;
mod eval;
mod explain;
mod verify;
mod config;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let cli: Cli = Cli::parse();
    cli.run().await
}
