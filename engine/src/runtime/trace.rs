use crate::{
    runtime::{EvalContext, EvaluationResult, Pattern, RuntimeError},
    value::RuntimeValue,
};
use std::{
    fmt::{Debug, Formatter},
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(feature = "monitor")]
use {super::monitor::dispatcher::Monitor, tokio::sync::Mutex};

/// Tracing information such as evaluation time.
#[derive(Debug, Clone, Copy)]
pub struct TraceResult {
    pub duration: Duration,
}

impl TraceResult {
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

#[derive(Clone)]
pub enum TraceConfig {
    #[cfg(feature = "monitor")]
    Enabled(Arc<Mutex<Monitor>>),
    Disabled,
}

impl Debug for TraceConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "monitor")]
            TraceConfig::Enabled(_) => {
                write!(f, "Trace::Enabled")
            }
            TraceConfig::Disabled => {
                write!(f, "Trace::Disabled")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TraceHandle<'ctx> {
    pub context: &'ctx EvalContext,
    pub ty: Arc<Pattern>,
    pub input: Arc<RuntimeValue>,
    pub start: Option<Instant>,
}

impl<'ctx> TraceHandle<'ctx> {
    pub(crate) fn run<'v>(
        self,
        block: Pin<Box<dyn Future<Output = Result<EvaluationResult, RuntimeError>> + 'v>>,
    ) -> Pin<Box<dyn Future<Output = Result<EvaluationResult, RuntimeError>> + 'v>>
    where
        'ctx: 'v,
    {
        if self.start.is_some() {
            Box::pin(async move {
                if let Some(correlation) = self.context.correlation().await {
                    self.context
                        .start(correlation, self.input.clone(), self.ty.clone())
                        .await;
                    let mut result = block.await;
                    let elapsed = self.start.map(|e| e.elapsed());
                    self.context
                        .complete(correlation, self.ty.clone(), &mut result, elapsed)
                        .await;
                    result
                } else {
                    block.await
                }
            })
        } else {
            block
        }
    }
}
