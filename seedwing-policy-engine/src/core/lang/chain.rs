/*
               let input = value.clone();
               let mut rationale = Vec::new();
               let mut cur = value;
               let mut cur_output = Output::None;
               for term in terms {
                   let result = term.evaluate(cur.clone(), bindings, world).await?;

                   rationale.push(result.clone());

                   match result.raw_output() {
                       Output::None => {
                           return Ok(EvaluationResult::new(
                               Some(cur.clone()),
                               self.clone(),
                               Rationale::Chain(rationale),
                               Output::None,
                           ));
                       }
                       Output::Identity => { /* keep trucking */ }
                       Output::Transform(val) => {
                           cur_output = Output::Transform(cur.clone());
                           cur = val.clone()
                       }
                   }
               }

               Ok(EvaluationResult::new(
                   Some(input),
                   self.clone(),
                   Rationale::Chain(rationale),
                   cur_output,
               ))
*/

use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext, InnerType};
use crate::runtime::rationale::Rationale;
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

const DOCUMENTATION: &str = include_str!("Chain.adoc");

const TERMS: &str = "terms";

#[derive(Debug)]
pub struct Chain;

impl Function for Chain {
    fn order(&self) -> u8 {
        128
    }
    fn parameters(&self) -> Vec<String> {
        vec![TERMS.into()]
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(terms) = bindings.get(TERMS) {
                if let InnerType::List(terms) = terms.inner() {
                    let original_input = input.clone();
                    let mut rationale = Vec::new();
                    let mut cur = input;
                    let mut cur_output = Output::None;
                    let mut last_output = Output::Identity;
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
