use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::package::Package;
use crate::runtime::World;
use crate::runtime::{EvalContext, PackagePath};
use crate::runtime::{Output, RuntimeError};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

pub fn package() -> Package {
    let mut pkg =
        Package::new(PackagePath::from_parts(vec!["cyclonedx"])).with_documentation(r#"Tools for working with CycloneDX

OWASP CycloneDX is a full-stack Bill of Materials (BOM) standard that provides advanced supply chain capabilities for cyber risk reduction. 
"#);
    pkg.register_source("v1_4".into(), include_str!("v1_4.dog"));
    //pkg.register_source("v1_4/structure".into(), include_str!("v1_4/v1_4.dog"));
    pkg.register_source("hash".into(), include_str!("hash.dog"));
    pkg.register_function("component-purls".into(), ComponentPurls);
    pkg
}

#[derive(Debug)]
pub struct ComponentPurls;

const DOCUMENTATION: &str = include_str!("component-purls.adoc");

impl Function for ComponentPurls {
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
            match input.as_json() {
                serde_json::Value::Object(o) => {
                    let mut purls = Vec::new();
                    if let Some(serde_json::Value::Array(components)) = o.get("components") {
                        for component in components.iter() {
                            if let serde_json::Value::Object(c) = component {
                                if let Some(serde_json::Value::String(s)) = c.get("purl") {
                                    purls.push(Arc::new(RuntimeValue::String(s.clone())));
                                }
                            }
                        }
                    }
                    Ok(Output::Transform(Arc::new(RuntimeValue::List(purls))).into())
                }
                _ => Ok(Severity::None.into()),
            }
        })
    }
}
