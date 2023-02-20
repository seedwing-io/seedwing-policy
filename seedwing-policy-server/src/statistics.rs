use std::sync::Arc;

use actix_web::http::header;
use actix_web::{get, rt, Error, HttpRequest};
use actix_web::{web, HttpResponse};
use handlebars::Handlebars;
use mime::Mime;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;

use seedwing_policy_engine::runtime::statistics::{Snapshot, Statistics};

use crate::ui::LAYOUT_HTML;

const STATISTICS_HTML: &str = include_str!("ui/_statistics.html");

#[get("/statistics/{path:.*}")]
pub async fn statistics(
    statistics: web::Data<Arc<Mutex<Statistics>>>,
    path: web::Path<String>,
    accept: web::Header<header::Accept>,
) -> HttpResponse {
    let path = path.replace('/', "::");

    let pref: Mime = accept.preference();

    if pref == mime::APPLICATION_JSON {
        statistics_json(path, statistics.get_ref().clone()).await
    } else {
        statistics_html(path).await
    }
}

pub async fn statistics_html(_path: String) -> HttpResponse {
    let mut renderer = Handlebars::new();
    renderer.set_prevent_indent(true);
    renderer.register_partial("layout", LAYOUT_HTML).unwrap();
    renderer
        .register_partial("statistics", STATISTICS_HTML)
        .unwrap();

    if let Ok(html) = renderer.render("statistics", &()) {
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

pub async fn statistics_json(_path: String, stats: Arc<Mutex<Statistics>>) -> HttpResponse {
    let snapshot = stats.lock().await.snapshot();
    HttpResponse::Ok().json(snapshot)
}

#[get("/metrics")]
pub async fn prometheus() -> HttpResponse {
    use ::prometheus::Encoder;
    let encoder = ::prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&::prometheus::default_registry().gather(), &mut buffer) {
        log::warn!("could not encode custom metrics: {}", e);
    };
    let res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("custom metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();
    HttpResponse::Ok().body(res)
}

#[get("/stream/statistics/{path:.*}")]
pub async fn statistics_stream(
    req: HttpRequest,
    stats: web::Data<Arc<Mutex<Statistics>>>,
    path: web::Path<String>,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    let path = path.replace('/', "::");
    let receiver = stats.lock().await.subscribe(path).await;
    // spawn websocket handler (and don't await it) so that the response is returned immediately
    rt::spawn(inner_statistics_stream(session, msg_stream, receiver));

    Ok(res)
}

pub async fn inner_statistics_stream(
    mut session: actix_ws::Session,
    _msg_stream: actix_ws::MessageStream,
    mut receiver: Receiver<Snapshot>,
) {
    loop {
        // todo! listen for close and other failures.
        if let Some(snapshot) = receiver.recv().await {
            if let Ok(json) = serde_json::to_string(&snapshot) {
                if session.text(json).await.is_err() {
                    // session closed
                }
            }
        }
    }
}
