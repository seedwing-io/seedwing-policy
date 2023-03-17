use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::rationale::Rationale;
use crate::runtime::World;
use crate::runtime::{EvalContext, Output, RuntimeError};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

#[derive(Debug)]
pub struct FromCve;

const DOCUMENTATION: &str = include_str!("from-cve.adoc");

impl Function for FromCve {
    fn order(&self) -> u8 {
        132
    }
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: Some(DOCUMENTATION.into()),
            ..Default::default()
        }
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let client = CvrfClient::new();
            match input.as_ref() {
                RuntimeValue::String(cve) => {
                    if let Ok(mut output) = client.find(cve).await {
                        let output: Vec<Arc<RuntimeValue>> = output
                            .drain(..)
                            .map(|v| Arc::new(RuntimeValue::String(v)))
                            .collect();
                        Ok(Output::Transform(Arc::new(RuntimeValue::List(output))).into())
                    } else {
                        Ok(Output::None.into())
                    }
                }
                _v => {
                    let msg = "input is not a string";
                    Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
                }
            }
        })
    }
}

pub struct CvrfClient {
    client: reqwest::Client,
}

impl CvrfClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn find(&self, cve: &str) -> Result<Vec<String>, anyhow::Error> {
        const BASE_URL: &str = "https://access.redhat.com/hydra/rest/securitydata/cvrf.json";
        let mut advisories = Vec::new();
        let response: serde_json::Value = self
            .client
            .get(format!("{}?cve={}", BASE_URL, cve))
            .send()
            .await?
            .json()
            .await?;
        if let Some(items) = response.as_array() {
            for item in items.iter() {
                if let Some(data) = item.as_object() {
                    if let Some(id) = data.get("RHSA") {
                        if let Some(id) = id.as_str() {
                            advisories.push(id.to_string());
                        }
                    }
                }
            }
        }
        Ok(advisories)
    }
}
