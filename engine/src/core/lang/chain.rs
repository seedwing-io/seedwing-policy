use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern};
use std::cmp::max;

use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
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
        ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(terms) = bindings.get(TERMS) {
                if let InnerPattern::List(terms) = terms.inner() {
                    let _original_input = input.clone();
                    let mut supporting = Vec::new();
                    let mut cur = input;
                    let mut cur_output = Output::Identity;
                    let mut cur_severity = Severity::None;

                    for term in terms {
                        let result = term
                            .evaluate(cur.clone(), ctx.push()?, bindings, world)
                            .await?;

                        let severity = result.severity();
                        let output = result.raw_output().clone();
                        supporting.push(result);

                        if matches!(severity, Severity::Error) {
                            return Ok((Severity::Error, Arc::new(supporting)).into());
                        }
                        cur_severity = max(cur_severity, severity);

                        match output {
                            Output::Identity => { /* keep trucking */ }
                            Output::Transform(val) => {
                                cur_output = Output::Transform(val.clone());
                                cur = val;
                            }
                        }
                    }

                    return Ok(FunctionEvaluationResult {
                        severity: cur_severity,
                        output: cur_output,
                        rationale: None,
                        supporting: Arc::new(supporting),
                    });
                }
            }
            Ok(Severity::Error.into())
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
