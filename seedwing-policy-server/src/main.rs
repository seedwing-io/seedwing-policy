mod api;
mod cli;
mod monitor;
mod playground;
mod policy;
mod statistics;
mod ui;

use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::{rt, web, App, HttpServer};
use actix_web_static_files::ResourceFiles;
use env_logger::Builder;
use log::LevelFilter;
use playground::PlaygroundState;
use seedwing_policy_engine::data::DirectoryDataSource;
use seedwing_policy_engine::error_printer::ErrorPrinter;
use static_files::Resource;
use std::collections::HashMap;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::Mutex;

use clap::Parser;
use seedwing_policy_engine::lang::builder::Builder as PolicyBuilder;
use seedwing_policy_engine::runtime::monitor::{Monitor, MonitorEvent};
use seedwing_policy_engine::runtime::sources::Directory;
use seedwing_policy_engine::runtime::statistics::Statistics;

use crate::cli::Cli;
use crate::policy::{display_component, display_root, display_root_no_slash, evaluate};
use crate::ui::{documentation, examples, index};

include!(concat!(env!("OUT_DIR"), "/generated-ROOT.rs"));
include!(concat!(env!("OUT_DIR"), "/generated-examples.rs"));
include!(concat!(env!("OUT_DIR"), "/generated-assets.rs"));
include!(concat!(env!("OUT_DIR"), "/generated-npm-assets.rs"));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli: Cli = Cli::parse();

    Builder::new()
        .filter_level(LevelFilter::Warn)
        .filter_module("seedwing_policy_server", cli.log.into())
        .filter_module("seedwing_policy_engine", cli.log.into())
        .init();

    let mut errors = Vec::new();

    let mut builder = PolicyBuilder::new();
    let mut sources = Vec::new();
    for dir in cli.policy_directories {
        if !dir.exists() {
            log::error!("Unable to open directory: {}", dir.to_string_lossy());
            exit(-3);
        }
        sources.push(Directory::new(dir));
    }

    for source in sources.iter() {
        if let Err(result) = builder.build(source.iter()) {
            errors.extend_from_slice(&result);
        }
    }

    if !errors.is_empty() {
        ErrorPrinter::new(builder.source_cache()).display(&errors);
        exit(-1)
    }

    for each in cli.data_directories {
        log::info!("loading data from {:?}", each);
        builder.data(DirectoryDataSource::new(each));
    }

    let result = builder.finish().await;

    let monitor = Arc::new(Mutex::new(Monitor::new()));

    let statistics = Arc::new(Mutex::new(Statistics::<100>::new(
        prometheus::default_registry(),
    )));

    match result {
        Ok(world) => {
            // todo: wire the receiver to a statistics gatherer.
            let mut receiver = monitor.lock().await.subscribe("".into()).await;

            let gatherer = statistics.clone();

            rt::spawn(async move {
                loop {
                    if let Some(result) = receiver.recv().await {
                        if let MonitorEvent::Complete(event) = &result {
                            if let Some(elapsed) = event.elapsed {
                                if let Some(name) = result.ty().name() {
                                    gatherer
                                        .lock()
                                        .await
                                        .record(name, elapsed, &event.completion)
                                        .await;
                                }
                            }
                        }
                    }
                }
            });

            let server = HttpServer::new(move || {
                let raw_docs = generate_docs();
                let raw_examples = generate_examples();
                let assets = generate_assets();
                let ui = generate_npm_assets();

                App::new()
                    .app_data(web::Data::new(world.clone()))
                    // use "from" in case of an existing Arc
                    .app_data(web::Data::from(monitor.clone()))
                    // use "from" in case of an existing Arc
                    .app_data(web::Data::from(statistics.clone()))
                    .app_data(web::Data::new(Documentation(raw_docs)))
                    .app_data(web::Data::new(Examples(raw_examples)))
                    .app_data(web::Data::new(PlaygroundState::new(
                        builder.clone(),
                        sources.clone(),
                    )))
                    .service(ResourceFiles::new("/assets", assets))
                    .service(ResourceFiles::new("/ui", ui))
                    .service(index)
                    .service(
                        web::scope("/api")
                            .wrap(NormalizePath::new(TrailingSlash::Always))
                            .service(api::get_policy)
                            .service(api::evaluate),
                    )
                    .service(display_root_no_slash)
                    .service(display_root)
                    .service(display_component)
                    .service(evaluate)
                    .service(documentation)
                    .service(examples)
                    .service(playground::display)
                    .service(playground::display_root_no_slash)
                    .service(playground::evaluate)
                    .service(playground::compile)
                    .service(monitor::monitor)
                    .service(monitor::monitor_stream)
                    .service(statistics::statistics)
                    .service(statistics::prometheus)
                    .service(statistics::statistics_stream)
            });
            log::info!("starting up at http://{}:{}/", cli.bind, cli.port);

            server.bind((cli.bind, cli.port))?.run().await
        }
        Err(errors) => {
            ErrorPrinter::new(builder.source_cache()).display(&errors);
            exit(-2);
        }
    }
}

pub struct Documentation(pub HashMap<&'static str, Resource>);

pub struct Examples(pub HashMap<&'static str, Resource>);
