use crate::Documentation;
use actix_web::http::header;
use actix_web::{get, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;
use serde::Serialize;
use std::str::from_utf8;

pub mod breadcrumbs;
pub mod html;
pub mod json;
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

#[get("/docs{path:.*}")]
pub async fn documentation(
    req: HttpRequest,
    path: web::Path<String>,
    docs: web::Data<Documentation>,
) -> HttpResponse {
    if path.is_empty() {
        let mut response = HttpResponse::TemporaryRedirect();
        response.insert_header((header::LOCATION, format!("{}/", req.path())));
        return response.finish();
    }

    let mut path: String = path.strip_prefix('/').unwrap_or(&*path).into();

    if path.ends_with(".png") {
        if let Some(image) = docs.0.get(path.as_str()) {
            let mut response = HttpResponse::Ok();
            response.insert_header((header::CONTENT_TYPE, "image/png"));
            return response.body(image.data);
        } else {
            return HttpResponse::InternalServerError().finish();
        }
    }

    let doc = docs.0.get(path.as_str());

    let doc = if doc.is_none() {
        if path.is_empty() {
            path = "index.adoc".into();
            docs.0.get(path.as_str())
        } else if path.ends_with('/') {
            path.push_str("index.adoc");
            docs.0.get(path.as_str())
        } else {
            let mut possible_redirect = path.clone();
            possible_redirect.push_str("/index.adoc");
            if docs.0.get(possible_redirect.as_str()).is_some() {
                let mut response = HttpResponse::TemporaryRedirect();
                response.insert_header((header::LOCATION, format!("{}/", req.path())));
                return response.finish();
            } else {
                path.push_str(".adoc");
                docs.0.get(path.as_str())
            }
        }
    } else {
        doc
    };

    if let Some(doc) = doc {
        if let Ok(content) = from_utf8(doc.data) {
            let mut renderer = Handlebars::new();
            renderer.set_prevent_indent(true);
            renderer.register_partial("layout", LAYOUT_HTML).unwrap();
            renderer
                .register_partial("documentation", DOCUMENTATION_HTML)
                .unwrap();
            let result = renderer.render(
                "documentation",
                &DocumentationContext {
                    content: content.into(),
                },
            );

            match result {
                Ok(html) => HttpResponse::Ok().body(html),
                Err(err) => {
                    log::error!("{:?}", err);
                    HttpResponse::InternalServerError().finish()
                }
            }
        } else {
            HttpResponse::InternalServerError().finish()
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[derive(Serialize)]
pub struct DocumentationContext {
    content: String,
}

pub(crate) const DOCUMENTATION_HTML: &str = include_str!("_documentation.html");
pub(crate) const LAYOUT_HTML: &str = include_str!("_layout.html");
pub(crate) const INDEX_HTML: &str = include_str!("_index.html");
