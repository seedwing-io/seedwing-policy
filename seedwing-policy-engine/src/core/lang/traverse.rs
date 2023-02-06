use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext, InnerType, ValueType};
use crate::runtime::{EvaluationResult, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("Traverse.adoc");

const STEP: &str = "step";

#[derive(Debug)]
pub struct Traverse;

impl Function for Traverse {
    fn order(&self) -> u8 {
        128
    }
    fn parameters(&self) -> Vec<String> {
        vec![STEP.into()]
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
            if let Some(step) = bindings.get(STEP) {
                if let InnerType::Const(ValueType::String(step)) = step.inner() {
                    if let Some(input) = input.try_get_object() {
                        if let Some(output) = input.get(step) {
                            return Ok(Output::Transform(output).into());
                        }
                    }
                }
            }

            Ok(Output::None.into())
        })
    }
}
