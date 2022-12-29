use crate::core::list::PATTERN;
use crate::core::{Function, FunctionError};
use crate::lang::lir::Bindings;
use crate::value::Value;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug)]
pub struct None;

impl Function for None {
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn call<'v>(
        &'v self,
        input: &'v Value,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<Value, FunctionError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list() {
                let pattern = bindings.get(PATTERN).unwrap();
                for item in list {
                    let result = pattern.evaluate(item.clone(), &Default::default()).await;

                    match result {
                        Ok(Option::None) => continue,
                        Err(_) => return Err(FunctionError::InvalidInput),
                        Ok(Option::Some(_)) => return Err(FunctionError::InvalidInput),
                    }
                }
                Ok(input.clone())
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
            type test-none = list::None<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 41, 43, 99]);

        let result = runtime.evaluate("test::test-none", value).await;

        assert!(matches!(result, Ok(Some(_)),))
    }

    #[actix_rt::test]
    async fn call_matching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-none = list::None<$(self > 50)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 49]);

        let result = runtime.evaluate("test::test-none", value).await;

        assert!(matches!(result, Ok(Some(_)),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-none = list::None<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99, 4, 2]);

        let result = runtime.evaluate("test::test-none", value).await;

        assert!(matches!(result, Ok(Option::None),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-none = list::None<$(self > 42)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime.evaluate("test::test-none", value).await;

        assert!(matches!(result, Ok(Option::None)))
    }

    #[actix_rt::test]
    async fn call_nonmatching_heterogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-none = list::None<$(self > 99.0)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-none", value).await;

        assert!(matches!(result, Ok(Option::None),))
    }

    #[actix_rt::test]
    async fn call_matching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-none = list::None<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-none", value).await;

        assert!(matches!(result, Ok(Option::Some(_)),))
    }

    #[actix_rt::test]
    async fn call_matching_empty() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-none = list::None<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([]);

        let result = runtime.evaluate("test::test-none", value).await;

        assert!(matches!(result, Ok(Option::Some(_)),))
    }
}
