use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerType};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

const DOCUMENTATION: &str = include_str!("Refine.adoc");

const REFINEMENT: &str = "refinement";

#[derive(Debug)]
pub struct Refine;

impl Function for Refine {
    fn parameters(&self) -> Vec<String> {
        vec![REFINEMENT.into()]
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
            let mut rationale = Vec::new();
            if let Some(refinement) = bindings.get(REFINEMENT) {
                let refinement_result = refinement.evaluate(input, bindings, world).await?;
                rationale.push(refinement_result.clone());
                if refinement_result.satisfied() {
                    return Ok((refinement_result.raw_output().clone(), rationale).into());
                } else {
                    return Ok((Output::None, rationale).into());
                }
            }

            return Ok((Output::None, rationale).into());
        })
    }
}
