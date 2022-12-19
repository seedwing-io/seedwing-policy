mod policy;

use actix_web::{web, App, Handler, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder};
use env_logger::Builder;
use log::LevelFilter;
use std::env;
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::policy::evaluate;
use seedwing_policy_engine::runtime::sources::Directory;
use seedwing_policy_engine::runtime::{Builder as PolicyBuilder, Runtime};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    Builder::new()
        .filter_level(LevelFilter::Warn)
        .filter_module("seedwing_proxy", LevelFilter::max())
        .init();

    let src = Directory::new(env::current_dir()?.join("policy"));

    println!("loading {:?}", src);
    let mut builder = PolicyBuilder::new();
    let result = builder.build(src.iter());
    let runtime = builder.link().await.unwrap();

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(runtime.clone()))
            .default_service(web::to(evaluate))
    });

    log::info!("running!");

    server.bind(("0.0.0.0", 8080))?.run().await
}