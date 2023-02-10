use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};

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
    async fn from_purl(
        &self,
        input: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, RuntimeError> {
        use serde_json::Value as JsonValue;
        let client = OsvClient::new();
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
                match client.query(&ecosystem, &name, &version).await {
                    Ok(transform) => {
                        let json: serde_json::Value = serde_json::to_value(transform).unwrap();
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
            match input.as_ref() {
                RuntimeValue::List(input) => {
                    let mut list: Vec<Arc<RuntimeValue>> = Vec::new();
                    for purl in input.iter() {
                        match self.call(purl.clone(), ctx, bindings, world).await {
                            Ok(result) => match result.output() {
                                Output::Transform(value) => {
                                    list.push(value);
                                }
                                _ => return Ok(result),
                            },
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }
                    Ok(Output::Transform(Arc::new(list.into())).into())
                }
                RuntimeValue::Object(input) => {
                    let input = input.as_json();
                    match self.from_purl(input).await {
                        Ok(Some(json)) => {
                            return Ok(Output::Transform(Arc::new(json.into())).into());
                        }
                        Ok(None) => Ok(Output::None.into()),
                        Err(e) => {
                            log::warn!("Error looking up {:?}", e);
                            Ok(Output::None.into())
                        }
                    }
                }
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
