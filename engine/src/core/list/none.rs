use crate::core::{list::PATTERN, Function, FunctionEvaluationResult};
use crate::lang::{lir::Bindings, Severity};
use crate::runtime::{rationale::Rationale, ExecutionContext, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
pub struct None;

impl Function for None {
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: ExecutionContext<'v>,
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
                            .evaluate(item.clone(), ctx.push()?, &Default::default(), world)
                            .await?,
                    );
                }

                match supporting.iter().all(|e| e.severity() == Severity::Error) {
                    true => Ok((Severity::None, supporting).into()),
                    false => Ok((Severity::Error, supporting).into()),
                }
            } else {
                Ok((Severity::Error, Rationale::NotAList).into())
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvalContext;
    use crate::{assert_not_satisfied, assert_satisfied};
    use serde_json::json;

    #[tokio::test]
    async fn call_matching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-none = list::none<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 41, 43, 99]);

        let result = runtime
            .evaluate("test::test-none", value, EvalContext::default())
            .await;

        assert_satisfied!(result.unwrap());
    }

    #[tokio::test]
    async fn call_matching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-none = list::none<$(self > 50)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 49]);

        let result = runtime
            .evaluate("test::test-none", value, EvalContext::default())
            .await;

        assert_satisfied!(result.unwrap());
    }

    #[tokio::test]
    async fn call_nonmatching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-none = list::none<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99, 4, 2]);

        let result = runtime
            .evaluate("test::test-none", value, EvalContext::default())
            .await;

        assert_not_satisfied!(result.unwrap());
    }

    #[tokio::test]
    async fn call_nonmatching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-none = list::none<$(self > 42)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime
            .evaluate("test::test-none", value, EvalContext::default())
            .await;

        assert_not_satisfied!(result.unwrap());
    }

    #[tokio::test]
    async fn call_nonmatching_heterogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-none = list::none<$(self > 99.0)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime
            .evaluate("test::test-none", value, EvalContext::default())
            .await;

        assert_not_satisfied!(result.unwrap());
    }

    #[tokio::test]
    async fn call_matching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-none = list::none<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 99, "Bob", 99.1]);

        let result = runtime
            .evaluate("test::test-none", value, EvalContext::default())
            .await;

        assert_satisfied!(result.unwrap());
    }

    #[tokio::test]
    async fn call_matching_empty() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-none = list::none<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([]);

        let result = runtime
            .evaluate("test::test-none", value, EvalContext::default())
            .await;

        assert_satisfied!(result.unwrap());
    }
}
