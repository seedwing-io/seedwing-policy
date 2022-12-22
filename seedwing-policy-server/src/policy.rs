use actix_web::http::Method;
use actix_web::web::{BytesMut, Payload};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use futures_util::stream::StreamExt;
use seedwing_policy_engine::runtime::Runtime;
use seedwing_policy_engine::value::Value;
use std::sync::Arc;

pub async fn evaluate(
    runtime: web::Data<Arc<Runtime>>,
    req: HttpRequest,
    mut body: Payload,
) -> impl Responder {
    if req.method() != Method::POST {
        return HttpResponse::NotAcceptable().finish();
    }

    let mut content = BytesMut::new();
    while let Some(Ok(bit)) = body.next().await {
        content.extend_from_slice(&bit);
    }

    // todo: accomodate non-JSON
    let result: Result<serde_json::Value, _> = serde_json::from_slice(&*content);

    if let Ok(result) = &result {
        let value = Value::from(result);
        let path = req.path().strip_prefix('/').unwrap().replace('/', "::");

        println!("{} {:?}", path, value);
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
