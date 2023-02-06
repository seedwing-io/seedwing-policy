use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("Iri.adoc");

#[derive(Debug)]
pub struct Iri;

impl Function for Iri {
    fn order(&self) -> u8 {
        128
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
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + Send + 'v>>
    {
        Box::pin(async move {
            if let Some(value) = input.try_get_string() {
                match ::iref::Iri::new(&value) {
                    Ok(_) => Ok(Output::Identity.into()),
                    Err(_) => Ok(Output::None.into()),
                }
            } else {
                Ok(Output::None.into())
            }
        })
    }
}
