use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern};

use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("chain.adoc");

const TERMS: &str = "terms";

#[derive(Debug)]
pub struct Chain;

impl Function for Chain {
    fn parameters(&self) -> Vec<String> {
        vec![TERMS.into()]
    }

    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: Some(DOCUMENTATION.into()),
            ..Default::default()
        }
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(terms) = bindings.get(TERMS) {
                if let InnerPattern::List(terms) = terms.inner() {
                    let _original_input = input.clone();
                    let mut rationale = Vec::new();
                    let mut cur = input;
                    let mut cur_output = Output::None;
                    let _last_output = Output::Identity;
                    for term in terms {
                        let result = term.evaluate(cur.clone(), ctx, bindings, world).await?;

                        rationale.push(result.clone());

                        match result.raw_output() {
                            Output::None => {
                                return Ok((Output::None, rationale).into());
                            }
                            Output::Identity => { /* keep trucking */ }
                            Output::Transform(val) => {
                                cur_output = Output::Transform(val.clone());
                                cur = val.clone()
                            }
                        }
                    }

                    return Ok((cur_output, rationale).into());
                }
            }

            Ok(Output::None.into())
        })
    }
}
