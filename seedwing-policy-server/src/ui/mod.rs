use actix_web::http::header;
use actix_web::{get, web, HttpResponse};

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

const LOGO_SVG: &[u8] = include_bytes!("logo.png");
