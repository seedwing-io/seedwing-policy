use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::rationale::Rationale;
use crate::runtime::World;
use crate::runtime::{EvalContext, Output, RuntimeError};
use crate::value::RuntimeValue;

use guac_rs::client::certify_vuln::allCertifyVuln;
use guac_rs::client::vulns2vex;

use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

#[derive(Debug)]
pub struct FromGuac;

const DOCUMENTATION: &str = include_str!("from-guac.adoc");

impl Function for FromGuac {
    fn order(&self) -> u8 {
        132
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            match input.as_ref() {
                RuntimeValue::List(items) => {
                    let mut vulns = Vec::new();
                    for item in items.iter() {
                        match serde_json::from_value::<allCertifyVuln>(item.as_json()) {
                            Ok(vuln) => {
                                vulns.push(vuln);
                            }
                            Err(e) => {
                                log::warn!("Error looking up {:?}", e);
                                return Ok(Output::None.into());
                            }
                        }
                    }

                    let vex = vulns2vex(vulns);
                    let json: serde_json::Value = serde_json::to_value(vex).unwrap();
                    Ok(Output::Transform(Arc::new(json.into())).into())
                }
                _v => {
                    let msg = "input is neither a Object nor a List";
                    Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
                }
            }
        })
    }
}
