//! Sharing monitoring results in memory between modules.
use crate::lang::lir::Pattern;
use crate::runtime::monitor::{CompleteEvent, Completion, MonitorEvent, StartEvent};
use crate::runtime::{Output, RuntimeError};
use crate::value::RuntimeValue;
use chrono::Utc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

pub struct Monitor {
    correlation: AtomicU64,
    subscribers: Arc<Mutex<Vec<Subscriber>>>,
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            correlation: AtomicU64::new(0),
            subscribers: Arc::new(Default::default()),
        }
    }

    pub async fn subscribe(&self, path: String) -> Receiver<MonitorEvent> {
        let (sender, receiver) = channel(500);
        self.subscribers.lock().await.push(Subscriber {
            path,
            sender,
            disconnected: false,
        });
        receiver
    }

    pub fn init(&self) -> u64 {
        self.correlation.fetch_add(1, Ordering::Relaxed)
    }

    pub async fn start(&self, correlation: u64, input: Arc<RuntimeValue>, ty: Arc<Pattern>) {
        let event = StartEvent {
            correlation,
            timestamp: Utc::now(),
            input,
            ty,
        };
        self.fanout(event.into()).await;
    }

    pub async fn complete_ok(
        &self,
        correlation: u64,
        ty: Arc<Pattern>,
        output: Output,
        elapsed: Option<Duration>,
    ) {
        let event = CompleteEvent {
            correlation,
            timestamp: Utc::now(),
            ty,
            completion: Completion::Output(output),
            elapsed,
        };
        self.fanout(event.into()).await;
    }

    pub async fn complete_err(
        &self,
        correlation: u64,
        ty: Arc<Pattern>,
        err: &RuntimeError,
        elapsed: Option<Duration>,
    ) {
        let event = CompleteEvent {
            correlation,
            timestamp: Utc::now(),
            ty,
            completion: Completion::Err(format!("{}", err)),
            elapsed,
        };
        self.fanout(event.into()).await;
    }

    async fn fanout(&self, event: MonitorEvent) {
        for subscriber in self
            .subscribers
            .lock()
            .await
            .iter_mut()
            .filter(|sub| sub.interested_in(event.ty()))
        {
            if let Err(err) = subscriber.sender.try_send(event.clone()) {
                match err {
                    TrySendError::Full(_) => {
                        // ehhh
                    }
                    TrySendError::Closed(_) => subscriber.disconnected = true,
                }
            }
        }

        let mut locked = self.subscribers.lock().await;
        let live_subscribers = locked.iter().filter(|e| !e.disconnected).cloned().collect();
        *locked = live_subscribers
    }
}

#[derive(Clone)]
pub struct Subscriber {
    path: String,
    sender: Sender<MonitorEvent>,
    disconnected: bool,
}

impl Subscriber {
    pub fn interested_in(&self, ty: Arc<Pattern>) -> bool {
        if let Some(name) = ty.name() {
            name.as_type_str().starts_with(&self.path)
        } else {
            false
        }
    }
}
