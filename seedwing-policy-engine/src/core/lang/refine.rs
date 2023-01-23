use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerType};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

const DOCUMENTATION: &str = include_str!("Refine.adoc");

const PRIMARY: &str = "primary";
const REFINEMENT: &str = "refinement";

#[derive(Debug)]
pub struct Refine;

impl Function for Refine {
    fn parameters(&self) -> Vec<String> {
        vec![PRIMARY.into(), REFINEMENT.into()]
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
            if let Some(primary) = bindings.get(PRIMARY) {
                let mut rationale = Vec::new();
                let primary_result = primary.evaluate(input.clone(), bindings, world).await?;
                rationale.push(primary_result.clone());

                if let Some(primary_output) = primary_result.output() {
                    if let Some(refinement) = bindings.get(REFINEMENT) {
                        let refinement_result =
                            refinement.evaluate(primary_output, bindings, world).await?;
                        rationale.push(refinement_result.clone());

                        if let Some(refinement_output) = refinement_result.output() {
                            return Ok(match primary_result.raw_output() {
                                Output::None => (Output::None, rationale).into(),
                                Output::Identity => (Output::Identity, rationale).into(),
                                Output::Transform(val) => {
                                    (Output::Transform(val.clone()), rationale).into()
                                }
                            });
                        }
                    }
                }

                return Ok((Output::None, rationale).into());
            }

            Ok(Output::None.into())
        })
    }
}
