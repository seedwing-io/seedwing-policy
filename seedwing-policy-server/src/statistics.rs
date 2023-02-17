use std::sync::Arc;
use std::time::Duration;

use actix_web::error::ParseError::Header;
use actix_web::get;
use actix_web::http::header::ACCEPT;
use actix_web::http::header::{self};
use actix_web::{web, HttpRequest, HttpResponse};
use handlebars::{Context, Handlebars, Helper, HelperResult, Output, RenderContext};
use mime::Mime;
use serde::Serialize;
use tokio::sync::Mutex;

use seedwing_policy_engine::runtime::statistics::{Statistics, TypeStats};

use crate::ui::LAYOUT_HTML;

const STATISTICS_HTML: &str = include_str!("ui/_statistics.html");

#[get("/statistics/{path:.*}")]
pub async fn statistics(
    req: HttpRequest,
    statistics: web::Data<Arc<Mutex<Statistics>>>,
    path: web::Path<String>,
    accept: web::Header<header::Accept>,
) -> HttpResponse {
    let path = path.replace('/', "::");

    let pref: Mime = accept.preference();

    if pref == mime::APPLICATION_JSON {
        statistics_json(path, statistics.get_ref().clone()).await
    } else {
        statistics_html(path, statistics.get_ref().clone()).await
    }
}

pub async fn statistics_html(path: String, stats: Arc<Mutex<Statistics>>) -> HttpResponse {
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

pub async fn statistics_json(path: String, stats: Arc<Mutex<Statistics>>) -> HttpResponse {
    let snapshot = stats.lock().await.snapshot();
    HttpResponse::Ok().json(snapshot)
}
