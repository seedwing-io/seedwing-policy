use crate::{
    runtime::{EvaluationResult, Pattern, RuntimeError},
    value::RuntimeValue,
};
use std::{
    fmt::{Debug, Formatter},
    future::Future,
    pin::Pin,
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "monitor")]
use {super::monitor::dispatcher::Monitor, std::time::Instant, tokio::sync::Mutex};

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

#[cfg(feature = "monitor")]
struct TraceRunner {
    pub monitor: Arc<Mutex<Monitor>>,
    pub input: Arc<RuntimeValue>,
    pub ty: Arc<Pattern>,
}

#[cfg(feature = "monitor")]
impl TraceRunner {
    async fn run(
        self,
        block: impl Future<Output = Result<EvaluationResult, RuntimeError>>,
    ) -> Result<EvaluationResult, RuntimeError> {
        let start = Instant::now();

        let correlation = {
            self.monitor
                .lock()
                .await
                .start(self.input, self.ty.clone())
                .await
        };

        let mut result = block.await;
        let elapsed = start.elapsed();

        match &mut result {
            Ok(result) => {
                result.with_trace_result(TraceResult { duration: elapsed });
                self.monitor
                    .lock()
                    .await
                    .complete_ok(
                        correlation,
                        self.ty,
                        result.severity(),
                        result.raw_output().clone(),
                        Some(elapsed),
                    )
                    .await;
            }
            Err(err) => {
                self.monitor
                    .lock()
                    .await
                    .complete_err(correlation, self.ty, &err, Some(elapsed))
                    .await;
            }
        }

        result
    }
}

#[derive(Clone, Debug)]
pub struct TraceContext(pub TraceConfig);

impl TraceContext {
    pub fn run<'v>(
        &self,
        #[allow(unused)] input: Arc<RuntimeValue>,
        #[allow(unused)] ty: Arc<Pattern>,
        block: Pin<Box<dyn Future<Output = Result<EvaluationResult, RuntimeError>> + 'v>>,
    ) -> Pin<Box<dyn Future<Output = Result<EvaluationResult, RuntimeError>> + 'v>> {
        match self.0.clone() {
            TraceConfig::Disabled => {
                return block;
            }
            #[cfg(feature = "monitor")]
            TraceConfig::Enabled(monitor) => {
                let runner = TraceRunner { monitor, input, ty };
                Box::pin(runner.run(block))
            }
        }
    }
}
