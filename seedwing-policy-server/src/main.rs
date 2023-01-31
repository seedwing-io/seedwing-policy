mod cli;
mod playground;
mod policy;
mod ui;

use actix_web::{web, App, HttpServer};
use actix_web_static_files::ResourceFiles;
use env_logger::Builder;
use log::LevelFilter;
use playground::PlaygroundState;
use seedwing_policy_engine::data::DirectoryDataSource;
use seedwing_policy_engine::error_printer::ErrorPrinter;
use static_files::Resource;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::exit;

use seedwing_policy_engine::lang::builder::Builder as PolicyBuilder;
use seedwing_policy_engine::runtime::sources::Directory;

use crate::cli::cli;
use crate::policy::{
    display_component, display_root, display_root_no_slash, evaluate_html, evaluate_json,
};
use crate::ui::{documentation, index};

include!(concat!(env!("OUT_DIR"), "/generated-docs.rs"));
include!(concat!(env!("OUT_DIR"), "/generated-assets.rs"));
include!(concat!(env!("OUT_DIR"), "/generated-npm-assets.rs"));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let command = cli();
    let matches = command.get_matches();

    let log_level: String = matches.get_one("log").cloned().unwrap();
    let filter = match log_level.as_str() {
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    Builder::new()
        .filter_level(LevelFilter::Warn)
        .filter_module("seedwing_policy_server", filter)
        .filter_module("seedwing_policy_engine", filter)
        .init();

    let bind: String = matches.get_one("bind").cloned().unwrap();
    let port: u16 = matches.get_one("port").cloned().unwrap();

    let mut errors = Vec::new();

    let mut builder = PolicyBuilder::new();
    let mut sources = Vec::new();
    if let Some(directories) = matches.get_many::<String>("dir") {
        for dir in directories {
            let dir = PathBuf::from(dir);
            if !dir.exists() {
                log::error!("Unable to open directory: {}", dir.to_string_lossy());
                exit(-3);
            }
            sources.push(Directory::new(dir));
        }
    }

    //log::info!("loading policies from {}", dir);
    for source in sources.iter() {
        if let Err(result) = builder.build(source.iter()) {
            errors.extend_from_slice(&result);
        }
    }

    if !errors.is_empty() {
        ErrorPrinter::new(builder.source_cache()).display(&errors);
        exit(-1)
    }

    if let Some(directories) = matches.get_many::<String>("data") {
        for each in directories {
            log::info!("loading data from {:?}", each);
            builder.data(DirectoryDataSource::new(each.into()));
        }
    }

    let result = builder.finish().await;

    match result {
        Ok(world) => {
            let server = HttpServer::new(move || {
                let raw_docs = generate_docs();
                let assets = generate_assets();
                let ui = generate_npm_assets();

                App::new()
                    .app_data(web::Data::new(world.clone()))
                    .app_data(web::Data::new(Documentation(raw_docs)))
                    .app_data(web::Data::new(PlaygroundState::new(
                        builder.clone(),
                        sources.clone(),
                    )))
                    .service(ResourceFiles::new("/assets", assets))
                    .service(ResourceFiles::new("/ui", ui))
                    .service(index)
                    .service(display_root_no_slash)
                    .service(display_root)
                    .service(display_component)
                    .service(evaluate_json)
                    .service(evaluate_html)
                    .service(documentation)
                    .service(playground::display)
                    .service(playground::display_root_no_slash)
                    .service(playground::evaluate)
                    .service(playground::compile)
            });
            log::info!("starting up at http://{}:{}/", bind, port);

            server.bind((bind, port))?.run().await
        }
        Err(errors) => {
            ErrorPrinter::new(builder.source_cache()).display(&errors);
            exit(-2);
        }
    }
}

pub struct Documentation(pub HashMap<&'static str, Resource>);
