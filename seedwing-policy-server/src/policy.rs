use crate::ui::LAYOUT_HTML;
use actix_web::http::header;
use actix_web::web::{BytesMut, Payload};
use actix_web::{get, post};
use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::stream::StreamExt;
use handlebars::Handlebars;
use seedwing_policy_engine::lang::lir::{Component, World};
use seedwing_policy_engine::lang::PackagePath;
use seedwing_policy_engine::value::Value;
use serde::Serialize;

/*
pub async fn policy(world: web::Data<World>, req: HttpRequest, body: Payload) -> impl Responder {
    if req.method() == Method::POST {
        return evaluate(world, req, body).await;
    }

    if req.method() == Method::GET {
        return display(world, req).await;
    }

    HttpResponse::NotAcceptable().finish()
}
 */

#[post("/policy/{path:.*}")]
pub async fn evaluate(
    req: HttpRequest,
    world: web::Data<World>,
    path: web::Path<String>,
    mut body: Payload,
) -> HttpResponse {
    let mut content = BytesMut::new();
    while let Some(Ok(bit)) = body.next().await {
        content.extend_from_slice(&bit);
    }

    // todo: accomodate non-JSON
    let result: Result<serde_json::Value, _> = serde_json::from_slice(&*content);

    if let Ok(result) = &result {
        let value = Value::from(result);
        //let path = req.path().strip_prefix('/').unwrap().replace('/', "::");

        match world.evaluate(&*path, value).await {
            Ok(result) => {
                if result.is_some() {
                    HttpResponse::Ok().finish()
                } else {
                    HttpResponse::NotAcceptable().finish()
                }
            }
            Err(_err) => HttpResponse::InternalServerError().finish(),
        }
    } else {
        HttpResponse::BadRequest().body(format!("Unable to parse POST'd input {}", req.path()))
    }
}

#[get("/policy")]
pub async fn display_root_no_slash(req: HttpRequest) -> HttpResponse {
    let mut response = HttpResponse::TemporaryRedirect();
    response.insert_header((header::LOCATION, format!("{}/", req.path())));
    response.finish()
}

#[get("/policy/")]
pub async fn display_root(world: web::Data<World>) -> HttpResponse {
    display(world, "".into()).await
}

#[get("/policy/{path:.*}")]
pub async fn display_component(world: web::Data<World>, path: web::Path<String>) -> HttpResponse {
    display(world, path.clone()).await
}

async fn display(world: web::Data<World>, path: String) -> HttpResponse {
    let original_path = path;
    let path = original_path.replace('/', "::");

    if let Some(component) = world.get(path.clone()) {
        let mut renderer = Handlebars::new();
        renderer.register_partial("layout", LAYOUT_HTML).unwrap();

        renderer.register_partial("module", MODULE_HTML).unwrap();

        renderer.register_partial("type", TYPE_HTML).unwrap();

        let result = match component {
            Component::Module(pkg) => {
                if !original_path.is_empty() && !original_path.ends_with('/') {
                    let mut response = HttpResponse::TemporaryRedirect();
                    response.insert_header((header::LOCATION, format!("{}/", path)));
                    return response.finish();
                }
                let path_segments = PackagePath::from(path.clone());
                let path_segments = path_segments.segments();
                renderer.render(
                    "module",
                    &RenderContext {
                        path_segments,
                        path,
                        payload: pkg,
                    },
                )
                //renderer.render_template(MODULE_HTML, &RenderContext { path, payload: pkg })
            }
            Component::Type(ty) => {
                renderer.render(
                    "type",
                    &RenderContext {
                        path_segments: vec![],
                        path,
                        payload: ty,
                    },
                )
                //renderer.render_template(TYPE_HTML, &RenderContext { path, payload: ty })
            }
        };

        match result {
            Ok(html) => HttpResponse::Ok().body(html),
            Err(err) => {
                log::error!("{:?}", err);
                HttpResponse::InternalServerError().finish()
            }
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

const TYPE_HTML: &str = include_str!("ui/_type.html");
const MODULE_HTML: &str = include_str!("ui/_module.html");

#[derive(Serialize)]
pub struct RenderContext<T: Serialize> {
    path_segments: Vec<String>,
    path: String,
    payload: T,
}
