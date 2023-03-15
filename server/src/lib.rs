#![deny(warnings)]

mod api;
mod cli;
mod metrics;
mod playground;
mod stream;
mod ui;

use actix_web::{web, App, HttpServer};
use playground::PlaygroundState;
use seedwing_policy_engine::data::DirectoryDataSource;
use seedwing_policy_engine::runtime::ErrorPrinter;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::Mutex;

use seedwing_policy_engine::lang::builder::Builder as PolicyBuilder;
use seedwing_policy_engine::runtime::monitor::dispatcher::Monitor;
use seedwing_policy_engine::runtime::monitor::MonitorEvent;
use seedwing_policy_engine::runtime::sources::Directory;
use seedwing_policy_engine::runtime::statistics::monitor::Statistics;

pub async fn run(
    policy_directories: Vec<PathBuf>,
    data_directories: Vec<PathBuf>,
    bind: String,
    port: u16,
) -> std::io::Result<()> {
    let mut errors = Vec::new();

    let mut builder = PolicyBuilder::new();
    let mut sources = Vec::new();
    for dir in policy_directories {
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

    for each in data_directories {
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

            tokio::spawn(async move {
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
                let app = App::new()
                    .app_data(web::Data::new(world.clone()))
                    // use "from" in case of an existing Arc
                    .app_data(web::Data::from(monitor.clone()))
                    // use "from" in case of an existing Arc
                    .app_data(web::Data::from(statistics.clone()))
                    .app_data(web::Data::new(PlaygroundState::new(
                        builder.clone(),
                        sources.clone(),
                    )));

                let app = app
                    .service(
                        web::scope("/api")
                            .service(api::openapi)
                            .service(api::get_policy)
                            .service(api::post_policy)
                            .service(api::evaluate)
                            .service(api::statistics)
                            .service(api::version),
                    )
                    .service(
                        web::scope("/stream")
                            .service(stream::statistics_stream)
                            .service(stream::monitor_stream),
                    )
                    .service(metrics::prometheus);

                #[cfg(feature = "frontend")]
                let app = {
                    use actix_web_static_files::ResourceFiles;

                    let app = app.service(web::scope("/openapi").service(
                        seedwing_policy_server_embedded_swaggerui::service("/api/openapi.json"),
                    ));

                    let spa = seedwing_policy_server_embedded_frontend::console_assets();
                    let spa = ResourceFiles::new("/", spa).resolve_not_found_to_root();
                    app.default_service(spa)
                };
                #[cfg(not(feature = "frontend"))]
                let app = {
                    use crate::ui::index;
                    app.service(index)
                };

                app
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
