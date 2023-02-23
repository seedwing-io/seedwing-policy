use actix_web::{get, rt, web, Error, HttpRequest, HttpResponse};
use seedwing_policy_engine::runtime::monitor::dispatcher::Monitor;
use seedwing_policy_engine::runtime::monitor::{MonitorEvent, SimpleMonitorEvent};
use seedwing_policy_engine::runtime::statistics::monitor::Statistics;
use seedwing_policy_engine::runtime::statistics::Snapshot;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;

#[get("/statistics/v1alpha1/{path:.*}")]
pub async fn statistics_stream(
    req: HttpRequest,
    stats: web::Data<Mutex<Statistics>>,
    path: web::Path<String>,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    let receiver = stats.lock().await.subscribe(path.clone()).await;
    // spawn websocket handler (and don't await it) so that the response is returned immediately
    rt::spawn(inner_statistics_stream(session, msg_stream, receiver));
    Ok(res)
}

pub async fn inner_statistics_stream(
    mut session: actix_ws::Session,
    _msg_stream: actix_ws::MessageStream,
    mut receiver: Receiver<Snapshot>,
) {
    loop {
        // todo! listen for close and other failures.
        if let Some(snapshot) = receiver.recv().await {
            if let Ok(json) = serde_json::to_string(&snapshot) {
                if session.text(json).await.is_err() {
                    // session closed
                }
            }
        }
    }
}

#[get("/monitor/v1alpha1/{path:.*}")]
pub async fn monitor_stream(
    req: HttpRequest,
    monitor_manager: web::Data<Mutex<Monitor>>,
    path: web::Path<String>,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    let receiver = monitor_manager.lock().await.subscribe(path.clone()).await;
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
            if let Ok(event) = SimpleMonitorEvent::try_from(event) {
                if let Ok(json) = serde_json::to_string(&event) {
                    if session.text(json).await.is_err() {
                        // session closed
                    }
                }
            }
        }
    }
}
