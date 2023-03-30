use crate::core::list::{COUNT, PATTERN};
use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{EvalContext, EvaluationResult, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::Severity;
use crate::runtime::rationale::Rationale;
use std::sync::Arc;

#[derive(Debug)]
pub struct Some;

impl Function for Some {
    fn parameters(&self) -> Vec<String> {
        vec![COUNT.into(), PATTERN.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let expected_count = match bindings.get(COUNT) {
                Option::Some(expected_count) => expected_count,
                Option::None => return Ok(Severity::Error.into()),
            };

            let pattern = match bindings.get(PATTERN) {
                Option::Some(pattern) => pattern,
                Option::None => return Ok(Severity::Error.into()),
            };

            if let Option::Some(list) = input.try_get_list() {
                let mut satisfied = false;
                let mut supporting: Vec<EvaluationResult> = Vec::new();
                let mut count = 0usize;

                // Fill the target until we reach the COUNT pattern, or more.
                // This means that we need to check every successful item, as otherwise
                // we may run over the target amount, which would also lead to a failed
                // COUNT check (as would "not enough").

                for item in list {
                    let item_result = pattern
                        .clone()
                        .evaluate(item.clone(), ctx, &Default::default(), world)
                        .await?;

                    supporting.push(item_result.clone());

                    let item_satisfied = item_result.severity() < Severity::Error;

                    // check if we now reached the target, but only if we didn't succeed it so far
                    if !satisfied && item_satisfied {
                        // we did succeed with this item, so count that
                        count += 1;

                        // now check
                        let expected_result = expected_count
                            .evaluate(
                                Arc::new((count as i64).into()),
                                ctx,
                                &Default::default(),
                                world,
                            )
                            .await?;

                        // record the outcome
                        satisfied = expected_result.severity() < Severity::Error;
                    }
                }

                let severity = match satisfied {
                    true => Severity::None,
                    false => Severity::Error,
                };

                Ok((severity, supporting).into())
            } else {
                Ok((Severity::Error, Rationale::NotAList).into())
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use crate::{assert_not_satisfied, assert_satisfied};
    use serde_json::json;

    #[tokio::test]
    async fn call_matching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-some = list::some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99, 42]);

        let result = runtime
            .evaluate("test::test-some", value, EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn call_matching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-some = list::some<2, $(self > 50)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99, 1024]);

        let result = runtime
            .evaluate("test::test-some", value, EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn call_matching_homogenous_type_more_than_necessary() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-some = list::some<2, $(self > 50)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99, 1024, 976,]);

        let result = runtime
            .evaluate("test::test-some", value, EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn call_nonmatching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-some = list::some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime
            .evaluate("test::test-some", value, EvalContext::default())
            .await
            .unwrap();

        assert_not_satisfied!(result);
    }

    #[tokio::test]
    async fn call_nonmatching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-some = list::some<2, $(self > 101)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 105, 99, 4, 2]);

        let result = runtime
            .evaluate("test::test-some", value, EvalContext::default())
            .await
            .unwrap();

        assert_not_satisfied!(result);
    }

    #[tokio::test]
    async fn call_matching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-some = list::some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 42, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime
            .evaluate("test::test-some", value, EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn call_matching_heterogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-some = list::some<2, $(self > 99.0)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1, 105]);

        let result = runtime
            .evaluate("test::test-some", value, EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn call_nonmatching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-some = list::some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 42, 4, 5, 99, "Bob", 99.1]);

        let result = runtime
            .evaluate("test::test-some", value, EvalContext::default())
            .await
            .unwrap();

        assert_not_satisfied!(result);
    }

    #[tokio::test]
    async fn call_nonmatching_empty() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-some = list::some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([]);

        let result = runtime
            .evaluate("test::test-some", value, EvalContext::default())
            .await
            .unwrap();

        assert_not_satisfied!(result);
    }
}
