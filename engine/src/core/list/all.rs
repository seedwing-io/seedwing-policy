use crate::core::{list::PATTERN, Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{EvalContext, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use crate::runtime::rationale::Rationale;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("all.adoc");

#[derive(Debug)]
pub struct All;

impl Function for All {
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
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

                let severity = supporting.iter().map(|s| s.severity()).collect();
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
            pattern test-all = list::all<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([42, 42, 42, 42, 42]);

        let result = runtime
            .evaluate("test::test-all", value, EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn call_matching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-all = list::all<$(self >= 42)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([43, 42, 49, 51, 42]);

        let result = runtime
            .evaluate("test::test-all", value, EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn call_nonmatching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-all = list::all<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([41, 42, 42, 42, 42]);

        let result = runtime
            .evaluate("test::test-all", value, EvalContext::default())
            .await
            .unwrap();

        assert_not_satisfied!(result);
    }

    #[tokio::test]
    async fn call_nonmatching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-all = list::all<$(self >= 42)>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3]);

        let result = runtime
            .evaluate("test::test-all", value, EvalContext::default())
            .await
            .unwrap();

        assert_not_satisfied!(result);
    }

    #[tokio::test]
    async fn call_matching_empty() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-all = list::all<42>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([]);

        let result = runtime
            .evaluate("test::test-all", value, EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(result);
    }
}
