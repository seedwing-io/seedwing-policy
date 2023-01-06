use crate::core::list::PATTERN;
use crate::core::{Function, FunctionError};
use crate::lang::lir::Bindings;
use crate::value::{InputValue, RationaleResult};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub struct All;

impl Function for All {
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn call<'v>(
        &'v self,
        input: Rc<InputValue>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<RationaleResult, FunctionError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list() {
                let pattern = bindings.get(PATTERN).unwrap();
                for item in list {
                    let result = pattern.evaluate(item.clone(), &Default::default()).await;

                    match result {
                        Ok(RationaleResult::None) => return Err(FunctionError::InvalidInput),
                        Err(_) => return Err(FunctionError::InvalidInput),
                        Ok(RationaleResult::Same(_) | RationaleResult::Transform(_)) => continue,
                    }
                }
                Ok(RationaleResult::Same(input.clone()))
            } else {
                Err(FunctionError::InvalidInput)
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
            type test-all = list::All<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([42, 42, 42, 42, 42]);

        let result = runtime.evaluate("test::test-all", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_matching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-all = list::All<$(self >= 42)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([43, 42, 49, 51, 42]);

        let result = runtime.evaluate("test::test-all", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-all = list::All<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([41, 42, 42, 42, 42]);

        let result = runtime.evaluate("test::test-all", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-all = list::All<$(self >= 42)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3]);

        let result = runtime.evaluate("test::test-all", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }

    #[actix_rt::test]
    async fn call_matching_empty() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-all = list::All<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([]);

        let result = runtime.evaluate("test::test-all", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }
}
