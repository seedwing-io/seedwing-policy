use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("length.adoc");

#[derive(Debug)]
pub struct Length;

impl Function for Length {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: Some(DOCUMENTATION.into()),
            ..Default::default()
        }
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(value) = input.try_get_string() {
                Ok(Output::Transform(Arc::new(value.len().into())).into())
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
    async fn call_matching_length() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern ten = string::length( $(self == 10) )
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate("test::ten", json!("abcdefghij"), EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_alias() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern ten = string::count( $(self == 10) )
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate("test::ten", json!("abcdefghij"), EvalContext::default())
            .await;

        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_non_matching_length() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern ten = string::length( $(self == 10) )
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate(
                "test::ten",
                json!("abcdefghijklmnop"),
                EvalContext::default(),
            )
            .await;

        println!("result --> {result:?}");

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_non_matching_not_a_string() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern ten = string::length( $(self == 10) )
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate("test::ten", json!(10), EvalContext::default())
            .await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(!result.unwrap().satisfied())
    }
}
