use crate::cli::Cli;

use clap::Parser;

mod bench;
mod cli;
mod eval;
mod explain;
mod verify;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let cli: Cli = Cli::parse();
    cli.run().await
}
