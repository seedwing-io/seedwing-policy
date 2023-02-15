use crate::{ui::rationale::Rationalizer, Documentation, PlaygroundState};
use actix_web::{
    get,
    http::header,
    post,
    web::{self},
    HttpResponse, Responder,
};
use seedwing_policy_engine::runtime::RuntimeError;
use seedwing_policy_engine::{
    lang::lir::EvalContext,
    runtime::{ComponentInformation, World},
    value::RuntimeValue,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[get("/policy/v1alpha1/{path:.*}")]
pub async fn get_policy(world: web::Data<World>, path: web::Path<String>) -> impl Responder {
    let path = path.into_inner().trim_matches('/').replace('/', "::");

    match world.get(path) {
        Some(component) => HttpResponse::Ok().json(&ComponentInformation::from(component)),
        None => HttpResponse::NotFound().finish(),
    }
}

#[get("/documentation/v1alpha1/{path:.*}")]
pub async fn get_documentation(
    docs: web::Data<Documentation>,
    path: web::Path<String>,
) -> impl Responder {
    let path = path.into_inner();
    let path = path.trim_matches('/');

    log::info!("Doc request: {path}");

    if path.ends_with(".png") {
        if let Some(image) = docs.0.get(path) {
            let mut response = HttpResponse::Ok();
            response.insert_header((header::CONTENT_TYPE, "image/png"));
            return response.body(image.data);
        } else {
            return HttpResponse::InternalServerError().finish();
        }
    }

    let doc = docs.0.get(path);

    let doc = if doc.is_none() {
        if path.is_empty() {
            // try main index file
            docs.0.get("index.adoc")
        } else if path.ends_with('/') {
            // try sub folder index file
            let mut path = path.to_string();
            path.push_str("index.adoc");
            docs.0.get(path.as_str())
        } else {
            // try content file
            let mut content_path = path.to_string();
            content_path.push_str(".adoc");
            let content = docs.0.get(content_path.as_str());
            if content.is_some() {
                content
            } else {
                let mut possible_redirect = path.to_string();
                possible_redirect.push_str("/index.adoc");
                docs.0.get(possible_redirect.as_str())
            }
        }
    } else {
        doc
    };

    match doc {
        Some(doc) => HttpResponse::Ok()
            .content_type("text/asciidoc")
            .body(doc.data),
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

    body: web::Json<EvaluateRequest>,
) -> HttpResponse {
    match state.build(body.policy.as_bytes()) {
        Ok(mut builder) => match builder.finish().await {
            Ok(world) => {
                let value = RuntimeValue::from(&body.value);
                match world
                    .evaluate(
                        format!("playground::{}", body.name),
                        value,
                        EvalContext::new(seedwing_policy_engine::lang::lir::EvalTrace::Enabled),
                    )
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
