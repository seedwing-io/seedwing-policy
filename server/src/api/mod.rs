use crate::playground::PlaygroundState;
use crate::ui::rationale::Rationalizer;
use actix_web::{
    get, post,
    web::{self},
    HttpResponse, Responder,
};
use seedwing_policy_engine::api::ToInformation;
use seedwing_policy_engine::runtime::statistics::monitor::Statistics;
use seedwing_policy_engine::{
    lang::lir::EvalContext,
    runtime::{monitor::dispatcher::Monitor, EvaluationResult, RuntimeError, World},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

#[get("/policy/v1alpha1/{path:.*}")]
pub async fn get_policy(world: web::Data<World>, path: web::Path<String>) -> impl Responder {
    let path = path.into_inner().trim_matches('/').replace('/', "::");

    match world.get(path) {
        Some(component) => match component.to_info(&world) {
            Ok(info) => HttpResponse::Ok().json(info),
            Err(err) => HttpResponse::InternalServerError().json(json!({
                "message": err.to_string(),
            })),
        },
        None => HttpResponse::NotFound().finish(),
    }
}

#[post("/policy/v1alpha1/{path:.*}")]
pub async fn post_policy(
    world: web::Data<World>,
    monitor: web::Data<Mutex<Monitor>>,
    path: web::Path<String>,
    value: web::Json<Value>,
) -> impl Responder {
    let path = path.into_inner().trim_matches('/').replace('/', "::");

    run_eval(monitor.into_inner(), &world, path, value.0).await
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EvaluateRequest {
    name: String,
    policy: String,
    value: Value,
}

#[post("/playground/v1alpha1/evaluate/")]
pub async fn evaluate(
    state: web::Data<PlaygroundState>,
    monitor: web::Data<Mutex<Monitor>>,
    body: web::Json<EvaluateRequest>,
) -> HttpResponse {
    match state.build(body.policy.as_bytes()) {
        Ok(mut builder) => match builder.finish().await {
            Ok(world) => {
                let EvaluateRequest {
                    name,
                    value,
                    policy: _,
                } = body.0;
                run_eval(
                    monitor.into_inner(),
                    &world,
                    format!("playground::{}", name),
                    value,
                )
                .await
            }
            Err(e) => {
                log::error!("err {:?}", e);
                let e = e
                    .iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                HttpResponse::BadRequest().body(e)
            }
        },
        Err(e) => {
            log::error!("unable to build policy [{:?}]", e);
            HttpResponse::NotAcceptable().body(e)
        }
    }
}

async fn run_eval(
    monitor: Arc<Mutex<Monitor>>,
    world: &World,
    path: String,
    value: Value,
) -> HttpResponse {
    let context = EvalContext::new(seedwing_policy_engine::lang::lir::TraceConfig::Enabled(
        monitor.clone(),
    ));

    match world.evaluate(path, value, context).await {
        Ok(result) => return_rationale(result),
        Err(RuntimeError::NoSuchPattern(name)) => HttpResponse::BadRequest().json(json!({
            "reason": "NoSuchPattern",
            "name": name.as_type_str(),
        })),
        Err(err) => {
            log::warn!("failed to run: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn return_rationale(result: EvaluationResult) -> HttpResponse {
    let rationale = Rationalizer::new(&result);
    let rationale = rationale.rationale();

    if result.satisfied() {
        HttpResponse::Ok().body(rationale)
    } else {
        HttpResponse::UnprocessableEntity().body(rationale)
    }
}

#[get("/statistics/v1alpha1/{path:.*}")]
pub async fn statistics(stats: web::Data<Mutex<Statistics>>) -> HttpResponse {
    let snapshot = stats.lock().await.snapshot();
    HttpResponse::Ok().json(snapshot)
}
