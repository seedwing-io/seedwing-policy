mod cli;
mod policy;

use actix_web::{web, App, HttpServer};
use clap::parser::ValuesRef;
use env_logger::Builder;
use log::LevelFilter;
use seedwing_policy_engine::error_printer::ErrorPrinter;
use std::path::PathBuf;
use std::process::exit;

use crate::policy::policy;
use seedwing_policy_engine::lang::builder::Builder as PolicyBuilder;
use seedwing_policy_engine::runtime::sources::Directory;

use crate::cli::cli;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    Builder::new()
        .filter_level(LevelFilter::Warn)
        .filter_module("seedwing_policy_server", LevelFilter::max())
        .filter_module("seedwing_policy_engine", LevelFilter::max())
        .init();

    let command = cli();
    let matches = command.get_matches();

    let bind: String = matches.get_one("bind").cloned().unwrap();
    let port: u16 = matches.get_one("port").cloned().unwrap();

    let mut errors = Vec::new();

    let mut builder = PolicyBuilder::new();
    let directories: ValuesRef<String> = matches.get_many("dir").unwrap();
    for dir in directories {
        let dir: PathBuf = dir.into();
        if !dir.exists() {
            log::error!("Unable to open directory: {}", dir.to_string_lossy());
            exit(-3);
        }
        let src = Directory::new(dir);
        //log::info!("loading policies from {}", dir);
        if let Err(result) = builder.build(src.iter()) {
            errors.extend_from_slice(&*result);
        }
    }

    if !errors.is_empty() {
        ErrorPrinter::new(builder.source_cache()).display(errors);
        exit(-1)
    }

    let result = builder.finish().await;

    match result {
        Ok(world) => {
            let server = HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(world.clone()))
                    .default_service(web::to(policy))
            });

            log::info!("starting up at http://{}:{}/", bind, port);

            server.bind((bind, port))?.run().await
        }
        Err(errors) => {
            ErrorPrinter::new(builder.source_cache()).display(errors);
            exit(-2);
        }
    }
}
