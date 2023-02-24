use crate::cli::Cli;

use clap::Parser;

mod cli;
mod command;
mod util;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let cli: Cli = Cli::parse();
    cli.run().await
}
