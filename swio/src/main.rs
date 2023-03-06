use crate::cli::Cli;

use crate::error::CliError;
use clap::Parser;

mod cli;
mod command;
mod config;
mod error;
mod util;

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let mut cli = Cli::parse();
    cli.run().await
}
