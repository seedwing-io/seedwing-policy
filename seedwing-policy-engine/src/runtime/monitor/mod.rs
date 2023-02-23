use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::lang::lir::{EvalContext, TraceHandle, Type};
use crate::runtime::{EvaluationResult, Output, RuntimeError};
use crate::value::RuntimeValue;
use serde::{Deserialize, Serialize};

#[cfg(feature = "monitor")]
pub mod dispatcher;

#[derive(Debug, Clone)]
pub enum MonitorEvent {
    Start(StartEvent),
    Complete(CompleteEvent),
}

impl MonitorEvent {
    pub fn ty(&self) -> Arc<Type> {
        match self {
            MonitorEvent::Start(inner) => inner.ty.clone(),
            MonitorEvent::Complete(inner) => inner.ty.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StartEvent {
    pub correlation: u64,
    pub timestamp: DateTime<Utc>,
    pub input: Arc<RuntimeValue>,
    pub ty: Arc<Type>,
}

impl From<StartEvent> for MonitorEvent {
    fn from(event: StartEvent) -> Self {
        MonitorEvent::Start(event)
    }
}

impl From<CompleteEvent> for MonitorEvent {
    fn from(event: CompleteEvent) -> Self {
        MonitorEvent::Complete(event)
    }
}

#[derive(Debug, Clone)]
pub struct CompleteEvent {
    pub correlation: u64,
    pub timestamp: DateTime<Utc>,
    pub ty: Arc<Type>,
    pub completion: Completion,
    pub elapsed: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum Completion {
    Output(Output),
    Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "event")]
#[serde(rename_all = "lowercase")]
pub enum SimpleMonitorEvent {
    Start(SimpleMonitorStart),
    Complete(SimpleMonitorComplete),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleMonitorStart {
    pub correlation: u64,
    pub timestamp: String,
    pub name: Option<String>,
    pub input: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleMonitorComplete {
    pub correlation: u64,
    pub timestamp: String,
    pub output: SimpleOutput,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "value")]
pub enum SimpleOutput {
    None,
    Identity,
    Transform(serde_json::Value),
    Err(String),
}

impl TryFrom<MonitorEvent> for SimpleMonitorEvent {
    type Error = ();

    fn try_from(value: MonitorEvent) -> Result<Self, Self::Error> {
        match value {
            MonitorEvent::Start(inner) => Ok(SimpleMonitorEvent::Start(SimpleMonitorStart {
                correlation: inner.correlation,
                timestamp: inner.timestamp.to_rfc2822(),
                name: inner.ty.name().map(|e| e.as_type_str()),
                input: inner.input.as_json(),
            })),
            MonitorEvent::Complete(inner) => {
                Ok(SimpleMonitorEvent::Complete(SimpleMonitorComplete {
                    correlation: inner.correlation,
                    timestamp: inner.timestamp.to_rfc2822(),
                    output: match inner.completion {
                        Completion::Output(Output::None) => SimpleOutput::None,
                        Completion::Output(Output::Identity) => SimpleOutput::Identity,
                        Completion::Output(Output::Transform(val)) => {
                            SimpleOutput::Transform(val.as_json())
                        }
                        Completion::Err(err) => SimpleOutput::Err(err),
                    },
                }))
            }
        }
    }
}
