use actix_web::http::Method;
use actix_web::web::{BytesMut, Payload};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use futures_util::stream::StreamExt;
use seedwing_policy_engine::runtime::{Component, Runtime};
use seedwing_policy_engine::value::Value;
use std::sync::Arc;

pub async fn policy(
    runtime: web::Data<Arc<Runtime>>,
    req: HttpRequest,
    body: Payload,
) -> impl Responder {
    if req.method() == Method::POST {
        return evaluate(runtime, req, body).await;
    }

    if req.method() == Method::GET {
        return display(runtime, req).await;
    }

    HttpResponse::NotAcceptable().finish()
}

async fn evaluate(
    runtime: web::Data<Arc<Runtime>>,
    req: HttpRequest,
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
        let path = req.path().strip_prefix('/').unwrap().replace('/', "::");

        match runtime.evaluate(path, value).await {
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

async fn display(runtime: web::Data<Arc<Runtime>>, req: HttpRequest) -> HttpResponse {
    let path = req.path().strip_prefix('/').unwrap().replace('/', "::");

    if let Some(component) = runtime.get(path.clone()).await {
        let mut html = String::new();

        html.push_str("<html>");
        html.push_str("<head>");
        html.push_str(format!("<title>Seedwing Policy {}</title>", path).as_str());
        html.push_str("</head>");
        html.push_str("<body style='font-family: sans-serif'>");
        html.push_str(format!("<h1>Seedwing Policy {}</h1>", path).as_str());
        match component {
            Component::Module(pkg) => {
                html.push_str(
                    pkg.to_html().await.as_str()
                )
            }
            Component::Type(ty) => {
                html.push_str(
                    ty.ty().await.to_html().await.as_str()
                );
            }
        }

        html.push_str("</body>");
        html.push_str("</html>");
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::NotFound().finish()
    }
}
