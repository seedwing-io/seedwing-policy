use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::rationale::Rationale;
use crate::runtime::World;
use crate::runtime::{EvalContext, Output, RuntimeError};
use crate::value::RuntimeValue;

use super::AdvisoryId;
use csaf::Csaf;
use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

#[derive(Debug)]
pub struct FindAdvisory;

const DOCUMENTATION: &str = include_str!("find-advisory.adoc");

impl Function for FindAdvisory {
    fn order(&self) -> u8 {
        132
    }
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
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
            let client = RhsaClient::new();
            match input.as_ref() {
                RuntimeValue::String(rhsa) => {
                    if let Ok(id) = rhsa.parse::<AdvisoryId>() {
                        if let Ok(Some(output)) = client.find(&id).await {
                            // TODO: Fix error type
                            let json = serde_json::to_value(output)
                                .map_err(|_| RuntimeError::InvalidState)?;
                            return Ok(Output::Transform(Arc::new(json.into())).into());
                        }
                    }
                    Ok(Severity::Error.into())
                }
                _v => {
                    let msg = "input is not a string";
                    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
                }
            }
        })
    }
}

pub struct RhsaClient {
    client: reqwest::Client,
}

impl RhsaClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn find(&self, id: &AdvisoryId) -> Result<Option<Csaf>, anyhow::Error> {
        const BASE_URL: &str = "https://access.redhat.com/security/data/csaf/v2/advisories";
        let (atype, year, number) = id.unwrap();
        let url = format!(
            "{}/{}/{}-{}_{:04}.json",
            BASE_URL, year, atype, year, number
        );

        let response = self.client.get(url).send().await?;
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Ok(None)
        }
    }
}
