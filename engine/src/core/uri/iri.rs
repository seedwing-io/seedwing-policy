use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("iri.adoc");

#[derive(Debug)]
pub struct Iri;

impl Function for Iri {
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
            if let Some(value) = input.try_get_string() {
                match ::iref::Iri::new(&value) {
                    Ok(_) => Ok(Output::Identity.into()),
                    Err(_) => Ok(Severity::Error.into()),
                }
            } else {
                Ok(Severity::Error.into())
            }
        })
    }
}
