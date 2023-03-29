use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;

use crate::runtime::{EvalContext, World};
use crate::runtime::{Output, RuntimeError};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

use super::client::*;

#[derive(Debug)]
pub struct ScanPurl;

const DOCUMENTATION: &str = include_str!("scan-purl.adoc");

fn json_to_query(input: serde_json::Value) -> Option<OsvQuery> {
    use serde_json::Value as JsonValue;
    match input {
        JsonValue::String(purl) => {
            let payload: OsvQuery = OsvQuery::from(purl.as_str());
            Some(payload)
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
                    Some(payload)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

impl ScanPurl {
    async fn from_purls(
        input: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, RuntimeError> {
        use serde_json::Value as JsonValue;
        let client = OsvClient::new();
        match input {
            JsonValue::Array(mut items) => {
                let queries: Vec<OsvQuery> = items.drain(..).flat_map(json_to_query).collect();

                log::info!("Batch queries: {}", queries.len());
                match client.query_batch(&queries).await {
                    Ok(mut result) => {
                        let mut vulns: Vec<OsvVulnerability> =
                            result.results.drain(..).flat_map(|v| v.vulns).collect();
                        let mut processed = Vec::new();
                        for vuln in vulns.drain(..) {
                            match client.fetch_id(vuln.id.as_str()).await {
                                Ok(vuln) => {
                                    processed.push(vuln);
                                }
                                Err(_e) => {
                                    // Fallback to existing info
                                    processed.push(vuln);
                                }
                            }
                        }
                        let vulns = processed;
                        let json: serde_json::Value =
                            serde_json::to_value(OsvResponse { vulns }).unwrap();
                        Ok(Some(json))
                    }
                    Err(e) => {
                        log::warn!("{:?}", e);
                        Ok(None)
                    }
                }
            }
            input => match json_to_query(input) {
                Some(query) => match client.query(query).await {
                    Ok(transform) => {
                        let json: serde_json::Value = serde_json::to_value(transform).unwrap();
                        Ok(Some(json))
                    }
                    Err(e) => {
                        log::warn!("Error looking up {:?}", e);
                        Ok(None)
                    }
                },
                _ => Ok(None),
            },
        }
    }
}

impl Function for ScanPurl {
    fn order(&self) -> u8 {
        // Reaching out to the network
        200
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
            match ScanPurl::from_purls(input.as_json()).await {
                Ok(Some(json)) => Ok(Output::Transform(Arc::new(json.into())).into()),
                _ => Ok(Severity::Error.into()),
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
