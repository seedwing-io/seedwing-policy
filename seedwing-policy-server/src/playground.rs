use crate::ui::rationale::Rationalizer;
use crate::ui::LAYOUT_HTML;
use actix_web::http::header;
use actix_web::web::{BytesMut, Payload};
use actix_web::{get, post};
use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::stream::StreamExt;
use handlebars::Handlebars;
use seedwing_policy_engine::lang::builder::Builder as PolicyBuilder;
use seedwing_policy_engine::lang::lir::EvalContext;
use seedwing_policy_engine::runtime::sources::{Directory, Ephemeral};
use seedwing_policy_engine::value::RuntimeValue;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct PlaygroundState {
    builder: PolicyBuilder,
    sources: Vec<Directory>,
}

impl PlaygroundState {
    pub fn new(builder: PolicyBuilder, sources: Vec<Directory>) -> Self {
        Self { builder, sources }
    }

    pub fn build(&self, policy: &[u8]) -> Result<PolicyBuilder, String> {
        let mut builder = self.builder.clone();
        for source in self.sources.iter() {
            if let Err(e) = builder.build(source.iter()) {
                log::error!("err {:?}", e);
                return Err(e
                    .iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<String>>()
                    .join(","));
            }
        }
        match core::str::from_utf8(policy) {
            Ok(s) => {
                if let Err(e) = builder.build(Ephemeral::new("playground", s).iter()) {
                    log::error!("unable to build policy [{:?}]", e);
                    return Err(format!("Compilation error: {e:?}"));
                }
            }
            Err(e) => {
                log::error!("unable to parse [{:?}]", e);
                return Err(format!("Unable to parse POST'd input {e:?}"));
            }
        }
        Ok(builder)
    }
}

#[get("/playground")]
pub async fn display_root_no_slash(req: HttpRequest) -> HttpResponse {
    let mut response = HttpResponse::TemporaryRedirect();
    response.insert_header((header::LOCATION, format!("{}/", req.path())));
    response.finish()
}

#[get("/playground/")]
pub async fn display(req: HttpRequest) -> HttpResponse {
    display_playground(req).await
}

#[post("/playground/compile")]
pub async fn compile(
    _req: HttpRequest,
    state: web::Data<PlaygroundState>,
    mut body: Payload,
) -> HttpResponse {
    let mut content = BytesMut::new();
    while let Some(Ok(bit)) = body.next().await {
        content.extend_from_slice(&bit);
    }

    match state.build(&content) {
        Ok(_) => HttpResponse::Ok().into(),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

#[derive(Deserialize, Debug)]
pub struct EvaluateRequest {
    policy: String,
    value: String,
}

#[post("/playground/evaluate/{path:.*}")]
pub async fn evaluate(
    req: HttpRequest,
    state: web::Data<PlaygroundState>,
    path: web::Path<String>,
    mut body: Payload,
) -> HttpResponse {
    let mut content = BytesMut::new();
    while let Some(Ok(bit)) = body.next().await {
        content.extend_from_slice(&bit);
    }
    match serde_json::from_slice::<EvaluateRequest>(&content) {
        Ok(body) => match serde_json::from_str::<serde_json::Value>(&body.value) {
            Ok(payload) => match state.build(body.policy.as_bytes()) {
                Ok(mut builder) => match builder.finish().await {
                    Ok(world) => {
                        let value = RuntimeValue::from(&payload);
                        let mut full_path = "playground::".to_string();
                        full_path += &path.replace('/', "::");

                        match world
                            .evaluate(
                                &*full_path,
                                value,
                                EvalContext::new(
                                    seedwing_policy_engine::lang::lir::EvalTrace::Enabled,
                                ),
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
                            Err(err) => {
                                log::error!("err {:?}", err);
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
            },
            Err(e) => {
                log::error!("unable to parse [{:?}]", e);
                HttpResponse::BadRequest()
                    .body(format!("Unable to parse POST'd input {}", req.path()))
            }
        },
        Err(e) => {
            log::error!("unable to parse [{:?}]", e);
            HttpResponse::BadRequest().body(format!("Unable to parse POST'd input {}", req.path()))
        }
    }
}

async fn display_playground(req: HttpRequest) -> HttpResponse {
    let compile_path = format!("{}compile", req.path());
    let eval_path = format!("{}evaluate", req.path());
    let mut renderer = Handlebars::new();
    renderer.set_prevent_indent(true);
    renderer.register_partial("layout", LAYOUT_HTML).unwrap();

    renderer
        .register_partial("playground", PLAYGROUND_HTML)
        .unwrap();
    /*
        response.insert_header((
            header::LOCATION,
            format!("{}/", path.strip_suffix('/').unwrap()),
        ));
        return response.finish();
    }*/

    let result = {
        //let path_segments = TypeName::from(path.clone());
        //let breadcrumbs = (path_segments()).into();

        //let html = Htmlifier::new("/playground/".into(), &*world);

        renderer.render(
            "playground",
            &PlaygroundRenderContext {
                //       breadcrumbs,
                compile_path,
                eval_path,
            },
        )
    };

    match result {
        Ok(html) => HttpResponse::Ok().body(html),
        Err(err) => {
            log::error!("{:?}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

const PLAYGROUND_HTML: &str = include_str!("ui/_playground.html");

#[derive(Serialize)]
pub struct PlaygroundRenderContext {
    //    breadcrumbs: Breadcrumbs,
    compile_path: String,
    eval_path: String,
    //    definition: String,
    //    documentation: String,
    //    parameters: Vec<String>,
}
