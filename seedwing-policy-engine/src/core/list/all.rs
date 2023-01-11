use crate::core::list::PATTERN;
use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{Output, RuntimeError};
use crate::value::{RationaleResult, RuntimeValue};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("All.adoc");

#[derive(Debug)]
pub struct All;

impl Function for All {
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list() {
                let pattern = bindings.get(PATTERN).unwrap();
                let mut supporting = Vec::new();
                for item in list {
                    supporting.push(pattern.evaluate(item.clone(), &Default::default()).await?);
                }

                if supporting.iter().all(|e| e.satisfied()) {
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
            pattern test-all = list::All<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([42, 42, 42, 42, 42]);

        let result = runtime.evaluate("test::test-all", value).await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_matching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-all = list::All<$(self >= 42)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([43, 42, 49, 51, 42]);

        let result = runtime.evaluate("test::test-all", value).await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-all = list::All<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([41, 42, 42, 42, 42]);

        let result = runtime.evaluate("test::test-all", value).await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-all = list::All<$(self >= 42)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3]);

        let result = runtime.evaluate("test::test-all", value).await;

        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_matching_empty() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-all = list::All<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([]);

        let ty = runtime.get("test::test-all");

        let result = runtime.evaluate("test::test-all", value).await;

        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
        assert!(result.unwrap().satisfied())
    }
}
