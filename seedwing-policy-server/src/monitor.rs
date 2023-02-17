use crate::ui::LAYOUT_HTML;
use actix_web::get;
use actix_web::Error;
use actix_web::{rt, web, HttpRequest, HttpResponse};
use handlebars::Handlebars;
use seedwing_policy_engine::runtime::monitor::{Completion, Monitor, MonitorEvent};
use seedwing_policy_engine::runtime::Output;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;

const MONITOR_HTML: &str = include_str!("ui/_monitor.html");

#[get("/monitor/{path:.*}")]
pub async fn monitor() -> HttpResponse {
    let mut renderer = Handlebars::new();
    renderer.set_prevent_indent(true);
    renderer.register_partial("layout", LAYOUT_HTML).unwrap();
    renderer.register_partial("monitor", MONITOR_HTML).unwrap();

    if let Ok(html) = renderer.render("monitor", &()) {
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

#[get("/monitor-stream/{path:.*}")]
pub async fn monitor_stream(
    req: HttpRequest,
    monitor_manager: web::Data<Arc<Mutex<Monitor>>>,
    path: web::Path<String>,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    let path = path.replace('/', "::");
    let receiver = monitor_manager.lock().await.subscribe(path).await;
    // spawn websocket handler (and don't await it) so that the response is returned immediately
    rt::spawn(inner_monitor_stream(session, msg_stream, receiver));

    Ok(res)
}

pub async fn inner_monitor_stream(
    mut session: actix_ws::Session,
    _msg_stream: actix_ws::MessageStream,
    mut receiver: Receiver<MonitorEvent>,
) {
    loop {
        // todo! listen for close and other failures.
        if let Some(event) = receiver.recv().await {
            if let Ok(event) = WsEvent::try_from(event) {
                if let Ok(json) = serde_json::to_string(&event) {
                    if session.text(json).await.is_err() {
                        // session closed
                    }
                }
            }
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", content = "event")]
#[serde(rename_all = "lowercase")]
pub enum WsEvent {
    Start(WsStart),
    Complete(WsComplete),
}

#[derive(Serialize)]
pub struct WsStart {
    correlation: u64,
    timestamp: String,
    name: Option<String>,
    input: Value,
}

#[derive(Serialize)]
pub struct WsComplete {
    correlation: u64,
    timestamp: String,
    output: WsOutput,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "value")]
pub enum WsOutput {
    None,
    Identity,
    Transform(Value),
    Err(String),
}

impl TryFrom<MonitorEvent> for WsEvent {
    type Error = ();

    fn try_from(value: MonitorEvent) -> Result<Self, Self::Error> {
        match value {
            MonitorEvent::Start(inner) => Ok(WsEvent::Start(WsStart {
                correlation: inner.correlation,
                timestamp: inner.timestamp.to_rfc2822(),
                name: inner.ty.name().map(|e| e.as_type_str()),
                input: inner.input.as_json(),
            })),
            MonitorEvent::Complete(inner) => Ok(WsEvent::Complete(WsComplete {
                correlation: inner.correlation,
                timestamp: inner.timestamp.to_rfc2822(),
                output: match inner.completion {
                    Completion::Output(Output::None) => WsOutput::None,
                    Completion::Output(Output::Identity) => WsOutput::Identity,
                    Completion::Output(Output::Transform(val)) => {
                        WsOutput::Transform(val.as_json())
                    }
                    Completion::Err(err) => WsOutput::Err(err.clone()),
                },
            })),
        }
    }
}
