use crate::cli::Cli;
use std::process::Termination;

use clap::Parser;

mod cli;
mod command;
mod config;
mod error;
mod util;

#[tokio::main]
async fn main() -> impl Termination {
    Cli::parse().run().await
}
