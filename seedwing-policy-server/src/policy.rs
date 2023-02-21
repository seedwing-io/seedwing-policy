use crate::ui::html::Htmlifier;
use crate::ui::{
    format::{parse, Format},
    LAYOUT_HTML,
};
use actix_web::http::header::{self};
use actix_web::web::Payload;
use actix_web::{get, post};
use actix_web::{web, HttpRequest, HttpResponse};
use handlebars::Handlebars;
use seedwing_policy_engine::lang::lir::TraceConfig;
//use seedwing_policy_engine::lang::lir::{Component, ModuleHandle, World};
//use seedwing_policy_engine::lang::{PackagePath, TypeName};
use crate::ui::breadcrumbs::Breadcrumbs;
use seedwing_policy_engine::lang::lir::EvalContext;
use seedwing_policy_engine::runtime::monitor::Monitor;
use seedwing_policy_engine::runtime::{Component, ModuleHandle, PackagePath, TypeName, World};
use seedwing_policy_engine::value::RuntimeValue;
use serde::Serialize;
use tokio::sync::Mutex;

#[derive(serde::Deserialize)]
pub struct PolicyQuery {
    opa: Option<bool>,
    collapse: Option<bool>,
    format: Option<Format>,
}

#[post("/policy/{path:.*}")]
pub async fn evaluate(
    world: web::Data<World>,
    monitor: web::Data<Mutex<Monitor>>,
    path: web::Path<String>,
    params: web::Query<PolicyQuery>,
    accept: web::Header<header::Accept>,
    mut body: Payload,
) -> HttpResponse {
    match &parse(&mut body).await {
        Ok(result) => {
            let value = RuntimeValue::from(result);
            let path = path.replace('/', "::");
            let trace = TraceConfig::Enabled(monitor.into_inner());
            match world.evaluate(&*path, value, EvalContext::new(trace)).await {
                Ok(result) => {
                    let mime = accept.preference();
                    let f = params.format.unwrap_or(mime.to_string().into());
                    let rationale = f.format(&result, params.collapse.unwrap_or(false));

                    if let Some(true) = params.opa {
                        // OPA result format
                        let satisfied = result.satisfied();
                        HttpResponse::Ok().json(serde_json::json!({ "result": satisfied }))
                    } else if result.satisfied() {
                        HttpResponse::Ok()
                            .content_type(f.content_type())
                            .body(rationale)
                    } else {
                        HttpResponse::UnprocessableEntity()
                            .content_type(f.content_type())
                            .body(rationale)
                    }
                }
                Err(err) => {
                    log::error!("err {:?}", err);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        Err(error) => HttpResponse::BadRequest().body(format!("{}", error)),
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

                let monitor_link = url_path.replace("/policy/", "/monitor/");

                renderer.render(
                    "module",
                    &ModuleRenderContext {
                        breadcrumbs,
                        url_path,
                        monitor_link,
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
                let monitor_link = url_path.replace("/policy/", "/monitor/");

                renderer.render(
                    "type",
                    &TypeRenderContext {
                        breadcrumbs,
                        url_path,
                        monitor_link,
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
    monitor_link: String,
    path: String,
    module: ModuleHandle,
}

#[derive(Serialize)]
pub struct TypeRenderContext {
    breadcrumbs: Breadcrumbs,
    url_path: String,
    monitor_link: String,
    path: String,
    definition: String,
    documentation: String,
    parameters: Vec<String>,
}
