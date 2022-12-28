use crate::core::{Function, FunctionError};
use crate::runtime::Bindings;
use crate::value::Value;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug)]
pub struct Any;

impl Function for Any {
    fn parameters(&self) -> Vec<String> {
        vec!["pattern".into()]
    }

    fn call<'v>(
        &'v self,
        input: &'v Value,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<Value, FunctionError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list() {
                let pattern = bindings.get(&"pattern".into()).unwrap();
                for item in list {
                    let result = pattern.evaluate(item.clone(), &Default::default()).await;

                    match result {
                        Ok(Some(_)) => return Ok(input.clone()),
                        Ok(None) => continue,
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
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::Builder;
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

        let runtime = builder.link().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(Some(_)),))
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

        let runtime = builder.link().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(Some(_)),))
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

        let runtime = builder.link().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(None),))
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

        let runtime = builder.link().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(None),))
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

        let runtime = builder.link().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(Some(_)),))
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

        let runtime = builder.link().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(Some(_)),))
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

        let runtime = builder.link().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-any", value).await;

        assert!(matches!(result, Ok(None),))
    }
}
