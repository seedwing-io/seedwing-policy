use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern, ValuePattern};
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("traverse.adoc");

const STEP: &str = "step";

#[derive(Debug)]
pub struct Traverse;

impl Function for Traverse {
    fn parameters(&self) -> Vec<String> {
        vec![STEP.into()]
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
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(step) = bindings.get(STEP) {
                if let InnerPattern::Const(ValuePattern::String(step)) = step.inner() {
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
