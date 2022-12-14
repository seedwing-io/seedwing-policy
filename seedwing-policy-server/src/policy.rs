use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use actix_web::dev::{HttpServiceFactory, Service};
use actix_web::{HttpRequest, HttpMessage, Responder, web, post, HttpResponse, Handler};
use actix_web::http::Method;
use actix_web::web::{BytesMut, Payload};
use actix_web::web::service;
use seedwing_policy_engine::runtime::{EvaluationResult, Runtime, RuntimeError};
use seedwing_policy_engine::value;
use futures_util::stream::StreamExt;
use serde_json::json;
use seedwing_policy_engine::value::Value;

pub async fn evaluate(runtime: web::Data<Arc<Runtime>>, mut req: HttpRequest, mut body: Payload) -> impl Responder {

    if req.method() != Method::POST {
        return HttpResponse::NotAcceptable().finish();
    }

    let mut content = BytesMut::new();
    while let Some(Ok(bit)) = body.next().await {
        content.extend_from_slice( &bit );
    }

    // todo: accomodate non-JSON
    let result: Result<serde_json::Value, _> = serde_json::from_slice( &*content);

    if let Ok(result) = &result {
        let mut value = Value::from(result);
        let path = req.path().strip_prefix("/").unwrap().replace("/", "::");

        println!("{} {:?}", path, value);
        match runtime.evaluate( path, &mut value ).await {
            Ok(result) => {
                if result.matches() {
                    HttpResponse::Ok().finish()
                } else {
                    HttpResponse::NotAcceptable().finish()
                }
            }
            Err(err) => {
                HttpResponse::InternalServerError().finish()
            }
        }
    } else {
        HttpResponse::BadRequest().body(
            format!("Unable to parse POST'd input {}", req.path())
        )
    }

}