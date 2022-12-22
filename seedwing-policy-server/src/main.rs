mod cli;
mod policy;

use actix_web::{web, App, HttpServer};
use clap::parser::ValuesRef;
use env_logger::Builder;
use log::LevelFilter;
use std::process::exit;

use crate::policy::evaluate;
use seedwing_policy_engine::runtime::sources::Directory;
use seedwing_policy_engine::runtime::Builder as PolicyBuilder;

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
        let src = Directory::new(dir.into());
        log::info!("loading policies from {}", dir);
        if let Err(result) = builder.build(src.iter()) {
            errors.extend_from_slice(&*result);
        }
    }

    if !errors.is_empty() {
        println!("{:?}", errors);
        exit(-1)
    }

    let result = builder.link().await;

    match result {
        Ok(runtime) => {
            let server = HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(runtime.clone()))
                    .default_service(web::to(evaluate))
            });

            log::info!("starting up at http://{}:{}/", bind, port);

            server.bind((bind, port))?.run().await
        }
        Err(errors) => {
            println!("{:?}", errors);
            exit(-2);
        }
    }
}
