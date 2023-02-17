use crate::{ui::rationale::Rationalizer, PlaygroundState};
use actix_web::{
    get, post,
    web::{self},
    HttpResponse, Responder,
};
use seedwing_policy_engine::{
    lang::lir::{EvalContext, Type},
    runtime::{monitor::Monitor, Component, ModuleHandle, RuntimeError, World},
    value::RuntimeValue,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ComponentInformation {
    Module(ModuleHandle),
    Type(TypeInformation),
}

impl From<Component> for ComponentInformation {
    fn from(value: Component) -> Self {
        match value {
            Component::Module(module) => Self::Module(module),
            Component::Type(r#type) => Self::Type(r#type.as_ref().into()),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TypeInformation {
    pub name: Option<String>,
    pub documentation: Option<String>,
    pub parameters: Vec<String>,
}

impl From<&Type> for TypeInformation {
    fn from(value: &Type) -> Self {
        Self {
            documentation: value.documentation(),
            parameters: value.parameters(),
            name: value.name().map(|name| name.as_type_str()),
        }
    }
}

#[get("/policy/v1alpha1/{path:.*}")]
pub async fn get_policy(world: web::Data<World>, path: web::Path<String>) -> impl Responder {
    let path = path.into_inner().trim_matches('/').replace('/', "::");

    match world.get(path) {
        Some(component) => HttpResponse::Ok().json(&ComponentInformation::from(component)),
        None => HttpResponse::NotFound().finish(),
    }
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
    monitor: web::Data<Arc<Mutex<Monitor>>>,
    body: web::Json<EvaluateRequest>,
) -> HttpResponse {
    let context = EvalContext::new(seedwing_policy_engine::lang::lir::TraceConfig::Enabled(
        monitor.get_ref().clone(),
    ));

    match state.build(body.policy.as_bytes()) {
        Ok(mut builder) => match builder.finish().await {
            Ok(world) => {
                let value = RuntimeValue::from(&body.value);
                match world
                    .evaluate(format!("playground::{}", body.name), value, context)
                    .await
                {
                    Ok(result) => {
                        let rationale = Rationalizer::new(&result);
                        let rationale = rationale.rationale();

                        if result.satisfied() {
                            HttpResponse::Ok().body(rationale)
                        } else {
                            HttpResponse::NotAcceptable().body(rationale)
                        }
                    }
                    Err(RuntimeError::NoSuchType(name)) => HttpResponse::BadRequest().json(json!({
                        "reason": "NoSuchType",
                        "name": name.as_type_str(),
                    })),
                    Err(err) => {
                        log::warn!("failed to run: {err}");
                        HttpResponse::InternalServerError().finish()
                    }
                }
            }
            Err(e) => {
                log::error!("err {:?}", e);
                let e = e
                    .iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                HttpResponse::BadRequest().body(e.to_string())
            }
        },
        Err(e) => {
            log::error!("unable to build policy [{:?}]", e);
            HttpResponse::NotAcceptable().body(e.to_string())
        }
    }
}
