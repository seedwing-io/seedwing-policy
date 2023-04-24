use crate::{
    client::RemoteClientBuilder,
    core::{Example, Function, FunctionEvaluationResult},
    lang::{lir::Bindings, PatternMeta, Severity, ValuePattern},
    runtime::{rationale::Rationale, ExecutionContext, Output, RuntimeError, World},
    value::RuntimeValue,
};
use std::{future::Future, pin::Pin, sync::Arc};
use url::Url;

const DOCUMENTATION: &str = include_str!("remote.adoc");

const URL: &str = "url";

#[derive(Debug)]
pub struct Remote {
    builder: RemoteClientBuilder,
}

impl Remote {
    pub fn new() -> Self {
        Self {
            builder: Default::default(),
        }
    }

    async fn execute(
        &self,
        input: &RuntimeValue,
        bindings: &Bindings,
    ) -> Result<FunctionEvaluationResult, RuntimeError> {
        // build the client
        let client = self.builder.build().await?;

        // get the URL
        let url = match bindings.get(URL).and_then(|p| p.try_get_resolved_value()) {
            Some(ValuePattern::String(url)) => url,
            _ => return error("Missing URL"),
        };

        let url = match Url::parse(&url) {
            Ok(url) => url,
            Err(err) => return error(format!("Invalid URL: {err}")),
        };

        // execute request
        let response = client.evaluate(url, input).await?;

        let output = match response.output {
            Some(output) => Output::Transform(Arc::new(output.into())),
            None => Output::Identity,
        };

        // convert the response
        let response = FunctionEvaluationResult {
            severity: response.severity,
            output,
            rationale: None,
            supporting: Arc::new(vec![]),
        };

        // done
        Ok(response)
    }
}

fn error(msg: impl Into<Arc<str>>) -> Result<FunctionEvaluationResult, RuntimeError> {
    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
}

impl Function for Remote {
    fn order(&self) -> u8 {
        192
    }

    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }

    fn parameters(&self) -> Vec<String> {
        vec![URL.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        // FIXME: propagate trace context, config context, and execution context
        Box::pin(async move { self.execute(&input, bindings).await })
    }
}
