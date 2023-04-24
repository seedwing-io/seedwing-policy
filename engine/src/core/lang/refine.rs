use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{ExecutionContext, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("refine.adoc");

const REFINEMENT: &str = "refinement";

#[derive(Debug)]
pub struct Refine;

impl Function for Refine {
    fn parameters(&self) -> Vec<String> {
        vec![REFINEMENT.into()]
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
        ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(refinement) = bindings.get(REFINEMENT) {
                let refinement_result = refinement
                    .evaluate(input, ctx.push()?, bindings, world)
                    .await?;
                let refinement_severity = refinement_result.severity();

                Ok(FunctionEvaluationResult {
                    severity: refinement_severity,
                    rationale: None,
                    output: refinement_result.raw_output().clone(),
                    supporting: Arc::new(vec![refinement_result]),
                })
            } else {
                Ok(Severity::Error.into())
            }
        })
    }
}
