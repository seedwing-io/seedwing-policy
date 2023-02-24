use crate::core::list::{COUNT, PATTERN};
use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::runtime::{EvaluationResult, Output, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

#[derive(Debug)]
pub struct Some;

impl Function for Some {
    fn order(&self) -> u8 {
        128
    }
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
            if let Option::Some(list) = input.try_get_list() {
                let expected_count = bindings.get(COUNT).unwrap();
                let pattern = bindings.get(PATTERN).unwrap();

                let mut supporting: Vec<EvaluationResult> = Vec::new();

                let mut satisfied = false;

                for item in list {
                    let item_result = pattern
                        .clone()
                        .evaluate(item.clone(), ctx, &Default::default(), world)
                        .await?;

                    supporting.push(item_result.clone());

                    if !satisfied && item_result.satisfied() {
                        let count = supporting.iter().filter(|e| e.satisfied()).count();
                        let expected_result = expected_count
                            .evaluate(
                                Arc::new((count as i64).into()),
                                ctx,
                                &Default::default(),
                                world,
                            )
                            .await?;
                        if expected_result.satisfied() {
                            satisfied = true
                        }
                    }
                }

                if satisfied {
                    Ok((Output::Identity, supporting).into())
                } else {
                    Ok((Output::None, supporting).into())
                }
            } else {
                Ok(Output::None.into())
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use serde_json::json;

    #[actix_rt::test]
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
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
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
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
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
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
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
            .await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
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
            .await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
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
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
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
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
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
            .await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
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
            .await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }
}
