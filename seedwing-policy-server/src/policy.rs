use actix_web::http::Method;
use actix_web::web::{BytesMut, Payload};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use futures_util::stream::StreamExt;
use handlebars::Handlebars;
use seedwing_policy_engine::lang::lir::{Component, World};
use seedwing_policy_engine::value::Value;

pub async fn policy(world: web::Data<World>, req: HttpRequest, body: Payload) -> impl Responder {
    if req.method() == Method::POST {
        return evaluate(world, req, body).await;
    }

    if req.method() == Method::GET {
        return display(world, req).await;
    }

    HttpResponse::NotAcceptable().finish()
}

async fn evaluate(world: web::Data<World>, req: HttpRequest, mut body: Payload) -> HttpResponse {
    let mut content = BytesMut::new();
    while let Some(Ok(bit)) = body.next().await {
        content.extend_from_slice(&bit);
    }

    // todo: accomodate non-JSON
    let result: Result<serde_json::Value, _> = serde_json::from_slice(&*content);

    if let Ok(result) = &result {
        let value = Value::from(result);
        let path = req.path().strip_prefix('/').unwrap().replace('/', "::");

        match world.evaluate(path, value).await {
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

async fn display(world: web::Data<World>, req: HttpRequest) -> HttpResponse {
    let path = req.path().strip_prefix('/').unwrap().replace('/', "::");

    if let Some(component) = world.get(path.clone()) {
        let mut renderer = Handlebars::new();
        renderer.set_dev_mode(true);

        let result = match component {
            Component::Module(pkg) => {
                renderer.render_template(MODULE_HTML, &RenderContext { path, payload: pkg })
            }
            Component::Type(ty) => {
                renderer.render_template(TYPE_HTML, &RenderContext { path, payload: ty })
            }
        };

        match result {
            Ok(html) => HttpResponse::Ok().body(html),
            Err(err) => {
                println!("{:?}", err);
                HttpResponse::InternalServerError().finish()
            }
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

const TYPE_HTML: &str = include_str!("ui/_type.html");
const MODULE_HTML: &str = include_str!("ui/_module.html");

use serde::Serialize;

#[derive(Serialize)]
pub struct RenderContext<T: Serialize> {
    path: String,
    payload: T,
}
