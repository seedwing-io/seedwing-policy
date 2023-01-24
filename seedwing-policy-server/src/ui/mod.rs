use actix_web::http::header;
use actix_web::{get, web, HttpResponse};
use handlebars::Handlebars;

pub mod breadcrumbs;
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

#[get("/_ui/{asset}")]
pub async fn ui_asset(path: web::Path<String>) -> HttpResponse {
    match &*path.into_inner() {
        "logo.png" => {
            let mut response = HttpResponse::Ok();
            response.insert_header((header::CONTENT_TYPE, "image/png"));
            response.body(LOGO_SVG)
        }
        _ => HttpResponse::NotFound().finish(),
    }
}

pub(crate) const LOGO_SVG: &[u8] = include_bytes!("logo.png");

pub(crate) const LAYOUT_HTML: &str = include_str!("_layout.html");
pub(crate) const INDEX_HTML: &str = include_str!("_index.html");
