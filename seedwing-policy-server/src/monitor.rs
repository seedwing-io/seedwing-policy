use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use actix_web::body::{BodySize, BodyStream};
use actix_web::{HttpRequest, HttpResponse, web};
use actix_web::web::Bytes;
use futures_util::Stream;
use actix_web::{get, post};
use actix_web::http::header;
use actix_web::http::header::{ACCEPT, HeaderValue};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use handlebars::Handlebars;
use tokio::sync::mpsc::{channel, Receiver};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use seedwing_policy_engine::runtime::{Component, TypeName, World};
use seedwing_policy_engine::value::RuntimeValue;
use crate::ui::LAYOUT_HTML;

const MONITOR_HTML: &str = include_str!("ui/_monitor.html");


#[get("/monitor/{path:.*}")]
pub async fn monitor(
    req: HttpRequest,
    world: web::Data<World>,
    monitor_manager: web::Data<Arc<Mutex<Monitor>>>,
    path: web::Path<String>,
) -> HttpResponse {
    let path = path.replace('/', "::");
    if let Some(Component::Type(_)) = world.get(path.clone()) {
        let name: TypeName = path.into();

        if let Some(value) = req.headers().get(ACCEPT) {
            if value.eq(&HeaderValue::from_static("text/event-stream")) {
                return monitor_sse(name, monitor_manager).await;
            }
        }
        let mut renderer = Handlebars::new();
        renderer.set_prevent_indent(true);
        renderer.register_partial("layout", LAYOUT_HTML).unwrap();
        renderer.register_partial("monitor", MONITOR_HTML).unwrap();

        if let Ok(html) = renderer.render(
            "monitor",
            &(),
        ) {
            HttpResponse::Ok().body(html)
        } else {
            HttpResponse::InternalServerError().finish()
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

pub async fn monitor_sse(name: TypeName, monitor_manager: web::Data<Arc<Mutex<Monitor>>>) -> HttpResponse {
    let mut resp = HttpResponse::Ok();
    resp.insert_header((header::CONTENT_TYPE, "text/event-stream"));
    resp.body(
        BodyStream::new(
            SseMonitor::new(
                monitor_manager.lock().await.monitor(name)
            )
        )
    )
}


#[derive(Clone, Debug)]
pub enum MonitorResult {
    Satisified,
    Unsatisfied,
    Error,
}

pub struct Monitor {
    registrations: Vec<Registration>,
}

#[derive(Clone, Debug)]
pub struct Registration {
    name: TypeName,
    sender: Sender<Entry>,
    stale: bool,
}

impl Registration {}

#[derive(Clone, Debug)]
pub struct Entry {
    name: TypeName,
    input: String,
    result: MonitorResult,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            registrations: vec![]
        }
    }

    pub async fn record<R: Into<MonitorResult>>(&mut self, name: TypeName, input: RuntimeValue, result: R) {
        if let Ok(input) = serde_json::to_string(&input) {
            let entry = Entry {
                name: name.clone(),
                input,
                result: result.into(),
            };

            println!("accept entry {:?}", entry);
            println!("accepted to regs now --> {:?}", self.registrations);

            for reg in &mut self.registrations.iter_mut() {
                println!("{} vs {}", name, reg.name);
                if name == reg.name {
                    if reg.sender.send(entry.clone()).await.is_err() {
                        reg.stale = true
                    }
                }
            }
        }

        let mut stale_purged = Vec::default();

        for reg in &self.registrations {
            if reg.stale {
                println!("remove registration {:?}", reg);
                // skip
            } else {
                stale_purged.push(reg.clone())
            }
        }

        self.registrations = stale_purged
    }

    pub fn monitor(&mut self, name: TypeName) -> impl Stream<Item=Entry> {
        let (sender, receiver) = channel(3);
        let reg = Registration {
            name,
            sender,
            stale: false,
        };

        println!("added registration {:?}", reg);

        self.registrations.push(reg);

        println!("regs now --> {:?}", self.registrations);

        ReceiverStream::new(receiver)
    }
}

pub struct SseMonitor<S: Stream<Item=Entry>> {
    stream: S,
}

impl<S: Stream<Item=Entry>> SseMonitor<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream
        }
    }
}

impl<S: Stream<Item=Entry> + Unpin> Stream for SseMonitor<S> {
    type Item = std::result::Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx).map(|e| {
            println!("polling... {:?}", e);
            e.map(|e| {
                println!("SSE an entry {:?}", e);
                let result = match e.result {
                    MonitorResult::Satisified => "satisfied",
                    MonitorResult::Unsatisfied => "unsatisfied",
                    MonitorResult::Error => "error",
                };

                let input = STANDARD.encode(e.input);
                Ok(Bytes::from(format!("data: {} {} {}\n\n", e.name.as_type_str(), result, input)))
            })
        })
    }
}