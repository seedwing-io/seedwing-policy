use crate::core::list::PATTERN;
use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("any.adoc");

#[derive(Debug)]
pub struct Any;

impl Function for Any {
    fn order(&self) -> u8 {
        128
    }
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list() {
                let pattern = bindings.get(PATTERN).unwrap();
                let mut supporting = Vec::new();
                for item in list {
                    supporting.push(
                        pattern
                            .evaluate(item.clone(), ctx, &Default::default(), world)
                            .await?,
                    );
                }

                if supporting.iter().any(|e| e.satisfied()) {
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
            pattern test-any = list::any<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99]);

        let result = runtime
            .evaluate("test::test-any", value, EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_matching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-any = list::any<$(self > 50)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99]);

        let result = runtime
            .evaluate("test::test-any", value, EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-any = list::any<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime
            .evaluate("test::test-any", value, EvalContext::default())
            .await;

        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-any = list::any<$(self > 101)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime
            .evaluate("test::test-any", value, EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_matching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-any = list::any<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime
            .evaluate("test::test-any", value, EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_matching_heterogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-any = list::any<$(self > 99.0)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime
            .evaluate("test::test-any", value, EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_nonmatching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-any = list::any<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 99, "Bob", 99.1]);

        let result = runtime
            .evaluate("test::test-any", value, EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_nonmatching_empty() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-any = list::any<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([]);

        let result = runtime
            .evaluate("test::test-any", value, EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_nested() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-any = list::any<
                list::any<99>
            >
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([[[]]]);

        let result = runtime
            .evaluate("test::test-any", value, EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }
}
