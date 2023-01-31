use crate::ui::html::Htmlifier;
use crate::ui::rationale::Rationalizer;
use crate::ui::{json, LAYOUT_HTML};
use actix_web::guard::{Acceptable, Any, Guard, GuardContext, Header};
use actix_web::http::header::{self};
use actix_web::web::{BytesMut, Payload};
use actix_web::{get, post};
use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::stream::StreamExt;
use handlebars::Handlebars;
use seedwing_policy_engine::lang::lir::EvalTrace;
//use seedwing_policy_engine::lang::lir::{Component, ModuleHandle, World};
//use seedwing_policy_engine::lang::{PackagePath, TypeName};
use crate::ui::breadcrumbs::Breadcrumbs;
use seedwing_policy_engine::lang::lir::EvalContext;
use seedwing_policy_engine::runtime::{
    Component, EvaluationResult, ModuleHandle, PackagePath, TypeName, World,
};
use seedwing_policy_engine::value::RuntimeValue;
use serde::Serialize;

#[derive(serde::Deserialize)]
pub struct PolicyQuery {
    opa: Option<bool>,
    trace: Option<bool>,
}

fn wants_json(ctx: &GuardContext) -> bool {
    Any(Acceptable::new(mime::APPLICATION_JSON))
        .or(Header(
            header::CONTENT_TYPE.as_str(),
            mime::APPLICATION_JSON.essence_str(),
        ))
        .check(ctx)
}

#[post("/policy/{path:.*}", guard = "wants_json")]
pub async fn evaluate_json(
    world: web::Data<World>,
    path: web::Path<String>,
    input: web::Json<serde_json::Value>,
    params: web::Query<PolicyQuery>,
) -> HttpResponse {
    let value = RuntimeValue::from(input.into_inner());
    let path = path.replace('/', "::");
    evaluate(world.get_ref(), path, value, params.into_inner(), |r| {
        serde_json::to_string_pretty(&json::Result::new(r)).unwrap()
    })
    .await
}

#[post("/policy/{path:.*}")]
pub async fn evaluate_html(
    req: HttpRequest,
    world: web::Data<World>,
    path: web::Path<String>,
    mut body: Payload,
    params: web::Query<PolicyQuery>,
) -> HttpResponse {
    let mut content = BytesMut::new();
    while let Some(Ok(bit)) = body.next().await {
        content.extend_from_slice(&bit);
    }

    // todo: accomodate non-JSON using content-type headers.
    let result: Result<serde_json::Value, _> = serde_json::from_slice(&content);

    if let Ok(result) = &result {
        let value = RuntimeValue::from(result);
        let path = path.replace('/', "::");

        evaluate(world.get_ref(), path, value, params.into_inner(), |r| {
            Rationalizer::new(r).rationale()
        })
        .await
    } else {
        log::error!("unable to parse [{:?}]", result);
        HttpResponse::BadRequest().body(format!("Unable to parse POST'd input {}", req.path()))
    }
}

async fn evaluate<F>(
    world: &World,
    path: String,
    value: RuntimeValue,
    params: PolicyQuery,
    formatter: F,
) -> HttpResponse
where
    F: Fn(&EvaluationResult) -> String,
{
    let mut trace = EvalTrace::Disabled;
    if let Some(true) = params.trace {
        trace = EvalTrace::Enabled;
    }
    match world.evaluate(&*path, value, EvalContext::new(trace)).await {
        Ok(result) => {
            let rationale = formatter(&result);

            if let Some(true) = params.opa {
                // OPA result format
                let satisfied = result.satisfied();
                HttpResponse::Ok().json(serde_json::json!({ "result": satisfied }))
            } else if result.satisfied() {
                HttpResponse::Ok().body(rationale)
            } else {
                HttpResponse::UnprocessableEntity().body(rationale)
            }
        }
        Err(err) => {
            log::error!("err {:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/policy")]
pub async fn display_root_no_slash(req: HttpRequest) -> HttpResponse {
    let mut response = HttpResponse::TemporaryRedirect();
    response.insert_header((header::LOCATION, format!("{}/", req.path())));
    response.finish()
}

#[get("/policy/")]
pub async fn display_root(req: HttpRequest, world: web::Data<World>) -> HttpResponse {
    display(req, world, "".into()).await
}

#[get("/policy/{path:.*}")]
pub async fn display_component(
    req: HttpRequest,
    world: web::Data<World>,
    path: web::Path<String>,
) -> HttpResponse {
    display(req, world, path.clone()).await
}

async fn display(req: HttpRequest, world: web::Data<World>, path: String) -> HttpResponse {
    let url_path = req.path().to_string();
    let original_path = path;
    let path = original_path.replace('/', "::");

    if let Some(component) = world.get(path.clone()) {
        let mut renderer = Handlebars::new();
        renderer.set_prevent_indent(true);
        renderer.register_partial("layout", LAYOUT_HTML).unwrap();

        renderer.register_partial("module", MODULE_HTML).unwrap();

        renderer.register_partial("type", TYPE_HTML).unwrap();

        let result = match component {
            Component::Module(module) => {
                if !original_path.is_empty() && !original_path.ends_with('/') {
                    let mut response = HttpResponse::TemporaryRedirect();
                    response.insert_header((header::LOCATION, format!("{path}/")));
                    return response.finish();
                }
                let path_segments = PackagePath::from(path.clone());
                let breadcrumbs = path_segments.into();

                renderer.render(
                    "module",
                    &ModuleRenderContext {
                        breadcrumbs,
                        url_path,
                        path,
                        module,
                    },
                )
            }
            Component::Type(ty) => {
                if original_path.ends_with('/') {
                    let mut response = HttpResponse::TemporaryRedirect();
                    response.insert_header((
                        header::LOCATION,
                        format!("{}/", path.strip_suffix('/').unwrap()),
                    ));
                    return response.finish();
                }
                let path_segments = TypeName::from(path.clone());
                let breadcrumbs = (path_segments, ty.parameters()).into();

                let html = Htmlifier::new("/policy/".into(), &world);

                renderer.render(
                    "type",
                    &TypeRenderContext {
                        breadcrumbs,
                        url_path,
                        path,
                        parameters: ty.parameters(),
                        documentation: ty.documentation().unwrap_or_default(),
                        definition: html.html_of(ty, &world),
                    },
                )
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
pub struct ModuleRenderContext {
    breadcrumbs: Breadcrumbs,
    url_path: String,
    path: String,
    module: ModuleHandle,
}

#[derive(Serialize)]
pub struct TypeRenderContext {
    breadcrumbs: Breadcrumbs,
    url_path: String,
    path: String,
    definition: String,
    documentation: String,
    parameters: Vec<String>,
}
