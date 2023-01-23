use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerType, ValueType};
use crate::runtime::{EvaluationResult, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

const DOCUMENTATION: &str = include_str!("Traverse.adoc");

const STEP: &str = "step";

#[derive(Debug)]
pub struct Traverse;

impl Function for Traverse {
    fn parameters(&self) -> Vec<String> {
        vec![STEP.into()]
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
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
