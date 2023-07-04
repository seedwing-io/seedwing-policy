use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::lang::{PatternMeta, Severity};
use crate::runtime::{ExecutionContext, World};
use crate::runtime::{Output, RuntimeError};
use crate::value::RuntimeValue;
use guac::client::GuacClient;
use packageurl::PackageUrl;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
pub struct CertifyVuln;

const DOCUMENTATION: &str = include_str!("certify-vulnerability.adoc");

fn json_to_pkg(input: serde_json::Value) -> Option<String> {
    use serde_json::Value as JsonValue;
    match input {
        JsonValue::String(purl) => Some(purl),
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
                ) => Some(
                    PackageUrl::new(r#type, name)
                        .unwrap()
                        .with_namespace(namespace)
                        .with_version(version)
                        .to_string(),
                ),
                _ => None,
            }
        }
        _ => None,
    }
}

impl CertifyVuln {
    async fn from_purls(
        input: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, RuntimeError> {
        use serde_json::Value as JsonValue;
        let guac = GuacClient::new("http://localhost:8080/query".to_string());
        match input {
            JsonValue::Array(items) => {
                let mut vulns = Vec::new();
                for item in items.iter() {
                    if let Some(value) = json_to_pkg(item.clone()) {
                        match guac.certify_vuln(&value).await {
                            Ok(transform) => {
                                let json: serde_json::Value =
                                    serde_json::to_value(transform).unwrap();
                                vulns.push(json);
                            }
                            Err(e) => {
                                log::warn!("Error looking up {:?}", e);
                            }
                        }
                    }
                }
                let json: serde_json::Value = serde_json::to_value(vulns).unwrap();
                Ok(Some(json))
            }
            input => match json_to_pkg(input) {
                Some(value) => match guac.certify_vuln(&value).await {
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

impl Function for CertifyVuln {
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
        _ctx: ExecutionContext<'v>,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            match CertifyVuln::from_purls(input.as_json()).await {
                Ok(Some(json)) => Ok(Output::Transform(Arc::new(json.into())).into()),
                _ => Ok(Severity::Error.into()),
            }
        })
    }
}
