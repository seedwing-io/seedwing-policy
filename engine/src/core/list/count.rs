use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("count.adoc");

#[derive(Debug)]
pub struct Count;

impl Function for Count {
    fn order(&self) -> u8 {
        128
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
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list() {
                Ok(Output::Transform(Arc::new(list.len().into())).into())
            } else {
                Ok(Output::Transform(Arc::new(0.into())).into())
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
    async fn list_count() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern count = list::count( $(self == 4) )
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let value = json!([1, 2, 3, 4]);
        let result = runtime
            .evaluate("test::count", value, EvalContext::default())
            .await;
        assert!(result.as_ref().unwrap().satisfied());
        assert_eq!(
            result
                .as_ref()
                .unwrap()
                .output()
                .unwrap()
                .try_get_integer()
                .unwrap(),
            4
        );
    }

    #[actix_rt::test]
    async fn list_length() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern len = list::length( $(self == 2) )
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let value = json!([1, 2]);
        let result = runtime
            .evaluate("test::len", value, EvalContext::default())
            .await;
        assert!(result.as_ref().unwrap().satisfied());
        assert_eq!(
            result
                .as_ref()
                .unwrap()
                .output()
                .unwrap()
                .try_get_integer()
                .unwrap(),
            2
        );
    }

    #[actix_rt::test]
    async fn list_count_none_list() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern count = list::length( $(self == 0) )
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let value = json!(123);
        let result = runtime
            .evaluate("test::count", value, EvalContext::default())
            .await;
        assert!(result.as_ref().unwrap().satisfied());
    }
}
