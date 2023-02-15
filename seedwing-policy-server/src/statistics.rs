use crate::ui::LAYOUT_HTML;
use actix_web::get;
use actix_web::{web, HttpRequest, HttpResponse};
use handlebars::Handlebars;
use seedwing_policy_engine::runtime::statistics::{Statistics, TypeStats};
use serde::Serialize;
use std::os::macos::raw::stat;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

const STATISTICS_HTML: &str = include_str!("ui/_statistics.html");

#[get("/statistics/{path:.*}")]
pub async fn statistics(
    req: HttpRequest,
    statistics: web::Data<Arc<Mutex<Statistics>>>,
    path: web::Path<String>,
) -> HttpResponse {
    let path = path.replace('/', "::");
    let mut renderer = Handlebars::new();
    renderer.set_prevent_indent(true);
    renderer.register_partial("layout", LAYOUT_HTML).unwrap();
    renderer
        .register_partial("statistics", STATISTICS_HTML)
        .unwrap();

    let mut stats = Vec::new();

    let snapshot = statistics.lock().await.snapshot();

    for (name, snap_stat) in snapshot {
        stats.push(WebStats {
            name: name.as_type_str(),
            invocations: snap_stat.invocations,
            mean_execution_time: format(&snap_stat.mean_execution_time),
        })
    }

    stats.sort_by(|l, r| l.name.cmp(&r.name));

    if let Ok(html) = renderer.render("statistics", &stats) {
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

#[derive(Serialize)]
pub struct WebStats {
    name: String,
    invocations: u64,
    mean_execution_time: String,
}

fn format(elapsed: &Duration) -> String {
    let ns = elapsed.as_nanos();

    let ms = ns / 1_000_000;
    let ns = ns % 1_000_000;

    let s = ms / 1_000;
    let ms = ms % 1_000;

    if s > 0 {
        format!("{}s {}ms", s, ms)
    } else if ms > 0 {
        format!("{}ms", ms)
    } else {
        format!("{}ns", ns)
    }
}
