mod api;
mod cli;
mod metrics;
mod playground;
mod stream;
mod ui;

use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::{rt, web, App, HttpServer};
use env_logger::Builder;
use log::LevelFilter;
use playground::PlaygroundState;
use seedwing_policy_engine::data::DirectoryDataSource;
use seedwing_policy_engine::runtime::ErrorPrinter;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::Mutex;

use clap::Parser;
use seedwing_policy_engine::lang::builder::Builder as PolicyBuilder;
use seedwing_policy_engine::runtime::monitor::dispatcher::Monitor;
use seedwing_policy_engine::runtime::monitor::MonitorEvent;
use seedwing_policy_engine::runtime::sources::Directory;
use seedwing_policy_engine::runtime::statistics::monitor::Statistics;
use seedwing_policy_server::run;

use crate::cli::Cli;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli: Cli = Cli::parse();

    Builder::new()
        .filter_level(LevelFilter::Warn)
        .filter_module("seedwing_policy_server", cli.log.into())
        .filter_module("seedwing_policy_engine", cli.log.into())
        .init();

    run(
        cli.policy_directories.clone(),
        cli.data_directories.clone(),
        cli.bind.clone(),
        cli.port,
    )
    .await
}
