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
            documentation: DOCUMENTATION.into(),
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
                    let mut cur_output = Output::Identity;
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

#[cfg(test)]
mod tests {
    use crate::{assert_satisfied, runtime::testutil::test_pattern};

    #[tokio::test]
    async fn chain_default_output() {
        let result = test_pattern("integer", 42).await;
        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn chain_identity_refine() {
        let result = test_pattern("integer | 42", 42).await;
        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn chain_transform() {
        let result = test_pattern(
            "string | uri::purl",
            "pkg:maven/org.apache.logging.log4j:log4j-core@2.14.0",
        )
        .await;
        assert_satisfied!(result);
    }
}
