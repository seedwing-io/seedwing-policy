use actix_web::{get, HttpResponse};
use handlebars::Handlebars;

pub mod breadcrumbs;
pub mod format;
pub mod html;
pub mod rationale;

#[get("/")]
pub async fn index() -> HttpResponse {
    let mut renderer = Handlebars::new();
    renderer.register_partial("layout", LAYOUT_HTML).unwrap();
    renderer.register_partial("index", INDEX_HTML).unwrap();

    let html = renderer.render("index", &());

    if let Ok(html) = html {
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

pub(crate) const LAYOUT_HTML: &str = include_str!("_layout.html");
pub(crate) const INDEX_HTML: &str = include_str!("_index.html");
