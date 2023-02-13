use anyhow::anyhow;

use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::runtime::rationale::Rationale;

use crate::runtime::World;
use crate::runtime::{Output, RuntimeError};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

use super::osv::*;

#[derive(Debug)]
pub struct FromPurl;

const DOCUMENTATION: &str = include_str!("from-purl.adoc");

impl FromPurl {
    async fn from_purls(
        &self,
        input: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, RuntimeError> {
        use serde_json::Value as JsonValue;
        let client = OsvClient::new();
        match input {
            JsonValue::Array(items) => {
                let queries: Vec<OsvQuery> = items
                    .iter()
                    .flat_map(|input| {
                        match (
                            input.get("name"),
                            input.get("namespace"),
                            input.get("type"),
                            input.get("version"),
                        ) {
                            (
                                Some(JsonValue::String(name)),
                                Some(JsonValue::String(namespace)),
                                Some(JsonValue::String(r#type)),
                                Some(JsonValue::String(version)),
                            ) => {
                                let (ecosystem, name) = purl2osv(r#type, name, namespace);
                                let payload: OsvQuery =
                                    (ecosystem, name.as_str(), version.as_str()).into();
                                Some(payload)
                            }
                            _ => None,
                        }
                    })
                    .collect();

                log::info!("Batch queries: {}", queries.len());
                match client.query_batch(queries).await {
                    Ok(result) => {
                        log::info!("Batch query response OK");
                        let json: serde_json::Value = serde_json::to_value(result.results).unwrap();
                        Ok(Some(json))
                    }
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Ok(None)
                    }
                }
            }
            JsonValue::Object(input) => {
                match (
                    input.get("name"),
                    input.get("namespace"),
                    input.get("type"),
                    input.get("version"),
                ) {
                    (
                        Some(JsonValue::String(name)),
                        Some(JsonValue::String(namespace)),
                        Some(JsonValue::String(r#type)),
                        Some(JsonValue::String(version)),
                    ) => {
                        let (ecosystem, name) = purl2osv(r#type, name, namespace);
                        let payload: OsvQuery = (ecosystem, name.as_str(), version.as_str()).into();
                        match client.query(payload).await {
                            Ok(transform) => {
                                let json: serde_json::Value =
                                    serde_json::to_value(transform).unwrap();
                                Ok(Some(json))
                            }
                            Err(e) => {
                                log::warn!("Error looking up {:?}", e);
                                Ok(None)
                            }
                        }
                    }
                    _ => Ok(None),
                }
            }
            _ => {
                log::warn!("Expected object or array JSON value");
                Ok(None)
            }
        }
    }
}

impl Function for FromPurl {
    fn order(&self) -> u8 {
        // Reaching out to the network
        200
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            match self.from_purls(input.as_json()).await {
                Ok(Some(json)) => Ok(Output::Transform(Arc::new(json.into())).into()),
                _ => Ok(Output::None.into()),
            }
        })
    }
}

fn purl2osv<'a>(r#type: &'a str, name: &str, namespace: &str) -> (&'a str, String) {
    let ecosystem = match r#type {
        "maven" => "Maven",
        "apk" => "Alpine",
        "cargo" => "crates.io",
        "deb" => "debian",
        "gem" => "RubyGems",
        "golang" => "Go",
        "nuget" => "NuGet",
        "pypi" => "PyPI",
        e => e,
    };

    let name = match r#type {
        "maven" => format!("{}:{}", namespace, name),
        "golang" => format!("{}/{}", namespace, name),
        "npm" => format!("{}/{}", namespace, name),
        _ => name.to_string(),
    };
    (ecosystem, name)
}
