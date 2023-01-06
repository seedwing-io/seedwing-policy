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
pub struct Any;

impl Function for Any {
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
                        Ok(RationaleResult::Same(_)) => {
                            return Ok(RationaleResult::Same(input.clone()))
                        }
                        Ok(RationaleResult::None) => continue,
                        _ => return Err(FunctionError::InvalidInput),
                    }
                }
                Err(FunctionError::InvalidInput)
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
            type test-any = list::Any<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_matching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-any = list::Any<$(self > 50)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-any = list::Any<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-any = list::Any<$(self > 101)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }

    #[actix_rt::test]
    async fn call_matching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-any = list::Any<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_matching_heterogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-any = list::Any<$(self > 99.0)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-any = list::Any<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_empty() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-any = list::Any<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }

    #[actix_rt::test]
    async fn call_nested() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-any = list::Any<
                list::Any<99>
            >
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([[[]]]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }
}
