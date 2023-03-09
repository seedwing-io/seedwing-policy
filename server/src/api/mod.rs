use crate::api::format::Format;
use crate::playground::PlaygroundState;
use actix_web::http::header;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::{
    get, post,
    web::{self},
    HttpResponse, Responder,
};
use okapi::openapi3::{
    Components, MediaType, OpenApi, Operation, PathItem, Ref, RefOr, RequestBody, SchemaObject,
};
use okapi::schemars::schema::{Metadata, Schema};
use seedwing_policy_engine::info::ToInformation;
use seedwing_policy_engine::runtime::config::EvalConfig;
use seedwing_policy_engine::runtime::statistics::monitor::Statistics;
use seedwing_policy_engine::runtime::{
    monitor::dispatcher::Monitor, EvalContext, EvaluationResult, RuntimeError, World,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

mod format;

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

#[derive(serde::Deserialize)]
pub struct PolicyQuery {
    opa: Option<bool>,
    collapse: Option<bool>,
    format: Option<Format>,
    minimal: Option<bool>,
}

#[derive(Copy, Clone)]
pub enum OutputEncoding {
    Seedwing { format: Format, collapse: bool },
    Opa,
}

impl Default for OutputEncoding {
    fn default() -> Self {
        Self::Seedwing {
            format: Format::Html,
            collapse: false,
        }
    }
}

impl OutputEncoding {
    fn from_request(accept: header::Accept, query: PolicyQuery) -> Self {
        if let Some(true) = query.opa {
            return OutputEncoding::Opa;
        }

        let mime = accept.preference();
        let mut format = query.format.unwrap_or(mime.to_string().into());
        if let Format::Json = format {
            if query.minimal.unwrap_or(false) {
                format = Format::JsonMinimal
            }
        }

        Self::Seedwing {
            format,
            collapse: query.collapse.unwrap_or_default(),
        }
    }
}

#[post("/policy/v1alpha1/{path:.*}")]
pub async fn post_policy(
    world: web::Data<World>,
    monitor: web::Data<Mutex<Monitor>>,
    path: web::Path<String>,
    accept: web::Header<header::Accept>,
    query: web::Query<PolicyQuery>,
    value: web::Json<Value>,
) -> impl Responder {
    let path = path.into_inner().trim_matches('/').replace('/', "::");

    let encoding = OutputEncoding::from_request(accept.into_inner(), query.into_inner());

    run_eval(monitor.into_inner(), &world, path, value.0, encoding).await
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
    accept: web::Header<header::Accept>,
    query: web::Query<PolicyQuery>,
) -> HttpResponse {
    let encoding = OutputEncoding::from_request(accept.into_inner(), query.into_inner());

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
                    encoding,
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
    encoding: OutputEncoding,
) -> HttpResponse {
    let context = EvalContext::new(
        seedwing_policy_engine::runtime::TraceConfig::Enabled(monitor.clone()),
        EvalConfig::default(),
    );

    match world.evaluate(path, value, context).await {
        Ok(result) => return_rationale(result, encoding),
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

fn return_rationale(result: EvaluationResult, encoding: OutputEncoding) -> HttpResponse {
    match encoding {
        OutputEncoding::Opa => {
            let satisfied = result.satisfied();
            HttpResponse::Ok().json(serde_json::json!({ "result": satisfied }))
        }
        OutputEncoding::Seedwing { format, collapse } => {
            let rationale = format.format(&result, collapse);

            if result.satisfied() {
                HttpResponse::Ok()
                    .content_type(format.content_type())
                    .body(rationale)
            } else {
                HttpResponse::UnprocessableEntity()
                    .content_type(format.content_type())
                    .body(rationale)
            }
        }
    }
}

#[get("/statistics/v1alpha1/{path:.*}")]
pub async fn statistics(stats: web::Data<Mutex<Statistics>>) -> HttpResponse {
    let snapshot = stats.lock().await.snapshot();
    HttpResponse::Ok().json(snapshot)
}

#[get("/swagger.json")]
pub async fn swagger(world: web::Data<World>) -> HttpResponse {
    let mut api = OpenApi::default();

    api.openapi = "3.0.0".into();
    api.info.title = "Seedwing Policy Server".into();
    api.info.version = "0.1".into();

    let mut schemas = okapi::Map::default();

    for (name, pattern) in world.all() {
        if !pattern.parameters().is_empty() {
            continue;
        }

        let path = format!("/api/policy/v1alpha1/{}", name.as_type_str());
        let mut path_item = PathItem::default();

        let mut get = Operation::default();

        get.description = Some("Retrieve the pattern definition".into());

        let mut post = Operation::default();

        post.description = pattern.documentation();
        let mut content = okapi::Map::new();
        let mut json_schema = pattern.as_json_schema(world.as_ref(), &vec![]);

        if let Schema::Object(mut json_schema) = json_schema {
            json_schema.metadata = Some(Box::new(Metadata {
                id: None,
                title: Some(name.as_type_str().into()),
                description: None,
                default: None,
                deprecated: false,
                read_only: false,
                write_only: false,
                examples: vec![],
            }));

            schemas.insert(name.as_type_str(), json_schema);

            let mut json_schema = SchemaObject::default();
            json_schema.reference = Some(format!("#/components/schemas/{}", name.as_type_str()));

            let json_media_type = MediaType {
                schema: Some(json_schema),
                example: None,
                examples: None,
                encoding: Default::default(),
                extensions: Default::default(),
            };

            content.insert("application/json".into(), json_media_type);

            let request_body = RefOr::Object(RequestBody {
                description: Some("The input value to evaluate against.".into()),
                content: content,
                required: true,
                extensions: Default::default(),
            });
            post.request_body = Some(request_body);

            path_item.get = Some(get);
            path_item.post = Some(post);
            api.paths.insert(path, path_item);
        }
    }

    let mut components = Components::default();
    components.schemas = schemas;
    api.components = Some(components);

    if let Ok(api) = serde_json::to_string_pretty(&api) {
        let mut response = HttpResponse::Ok();
        response.insert_header((CONTENT_TYPE, "application/json"));
        response.body(api)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}
