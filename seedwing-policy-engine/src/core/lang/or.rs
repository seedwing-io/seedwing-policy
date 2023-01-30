use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerType, Type, ValueType};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("Or.adoc");

const TERMS: &str = "terms";

#[derive(Debug)]
pub struct Or;

impl Function for Or {
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
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(terms) = bindings.get(TERMS) {
                if let InnerType::List(terms) = terms.inner() {
                    let mut rationale = Vec::new();
                    let mut terms = terms.clone();
                    terms.sort_by(|a, b| a.order(world).cmp(&b.order(world)));
                    for term in terms {
                        let result = term.evaluate(input.clone(), bindings, world).await?;
                        if result.satisfied() {
                            rationale.push(result);
                            return Ok((Output::Identity, rationale).into());
                        }
                        rationale.push(result);
                    }

                    return Ok((Output::None, rationale).into());
                } else {
                    let result = terms.evaluate(input.clone(), bindings, world).await?;
                    if let Some(terms) = result.output() {
                        if let RuntimeValue::List(terms) = &*terms {
                            let mut rationale = Vec::new();
                            for term in terms {
                                let term: Type = term.clone().into();
                                let result = Arc::new(term)
                                    .evaluate(input.clone(), bindings, world)
                                    .await?;
                                if result.satisfied() {
                                    rationale.push(result);
                                    return Ok((Output::Identity, rationale).into());
                                }
                                rationale.push(result);
                            }

                            return Ok((Output::None, rationale).into());
                        }
                    }
                }
            }

            Ok(Output::None.into())
        })
    }
}
