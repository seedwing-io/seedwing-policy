use crate::ui::LAYOUT_HTML;
use actix_web::get;
use actix_web::{web, HttpResponse};
use handlebars::{Context, Handlebars, Helper, HelperResult, Output, RenderContext};
use seedwing_policy_engine::runtime::statistics::Statistics;
use std::sync::Arc;
use tokio::sync::Mutex;

const STATISTICS_HTML: &str = include_str!("ui/_statistics.html");

#[get("/statistics/{path:.*}")]
pub async fn statistics(statistics: web::Data<Arc<Mutex<Statistics>>>) -> HttpResponse {
    let mut renderer = Handlebars::new();
    renderer.set_prevent_indent(true);
    renderer.register_partial("layout", LAYOUT_HTML).unwrap();
    renderer
        .register_partial("statistics", STATISTICS_HTML)
        .unwrap();
    renderer.register_helper("format-time", Box::new(format_time));

    let mut snapshot = statistics.lock().await.snapshot();
    snapshot.sort_by(|l, r| l.name.cmp(&r.name));

    if let Ok(html) = renderer.render("statistics", &snapshot) {
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

// implement via bare function
fn format_time(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let ns = h.param(0).unwrap();
    let ns = ns.value().as_u64().unwrap();
    let ms = ns / 1_000_000;
    let ns = ns % 1_000_000;

    let s = ms / 1_000;
    let ms = ms % 1_000;

    if s > 0 {
        out.write(format!("{}s {}ms", s, ms).as_str())?;
    } else if ms > 0 {
        out.write(format!("{}ms", ms).as_str())?;
    } else {
        out.write(format!("{}ns", ns).as_str())?;
    }

    Ok(())
}
