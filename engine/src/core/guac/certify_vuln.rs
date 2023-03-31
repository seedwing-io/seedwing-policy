use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;

use crate::runtime::{EvalContext, World};
use crate::runtime::{Output, RuntimeError};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use guac_rs::client::{certify_vuln::*, GuacClient};
use std::sync::Arc;

#[derive(Debug)]
pub struct CertifyVuln;

const DOCUMENTATION: &str = include_str!("certify-vulnerability.adoc");

fn json_to_pkg(input: serde_json::Value) -> Option<PkgSpec> {
    use serde_json::Value as JsonValue;
    match input {
        JsonValue::Object(input) => {
            let pkg = PkgSpec {
                type_: input.get("type").map(|val| val.to_string()),
                namespace: input.get("namespace").map(|val| val.to_string()),
                name: input.get("name").map(|val| val.to_string()),
                subpath: None,
                version: input.get("version").map(|val| val.to_string()),
                qualifiers: None, //TODO fix qualifiers
                match_only_empty_qualifiers: Some(false),
            };

            Some(pkg)
        }
        _ => {
            log::warn!("Unknown package spec {:?}", input);
            None
        }
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
                        match guac.certify_vuln(value).await {
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
                Some(value) => match guac.certify_vuln(value).await {
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
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            match CertifyVuln::from_purls(input.as_json()).await {
                Ok(Some(json)) => Ok(Output::Transform(Arc::new(json.into())).into()),
                _ => Ok(Output::None.into()),
            }
        })
    }
}
