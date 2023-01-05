use crate::core::list::{COUNT, PATTERN};
use crate::core::{Function, FunctionError};
use crate::lang::lir::Bindings;
use crate::runtime::EvaluationResult;
use crate::value::{RationaleResult, Value};
use async_mutex::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
pub struct Some;

impl Function for Some {
    fn parameters(&self) -> Vec<String> {
        vec![COUNT.into(), PATTERN.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<Mutex<Value>>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<RationaleResult, FunctionError>> + 'v>> {
        Box::pin(async move {
            let mut locked_input = input.lock().await;
            if let Option::Some(list) = locked_input.try_get_list() {
                let expected_count = bindings.get(COUNT).unwrap();
                let pattern = bindings.get(PATTERN).unwrap();

                let mut count: u32 = 0;

                for item in list {
                    let result = pattern
                        .clone()
                        .evaluate(item.clone(), &Default::default())
                        .await;

                    if let Ok(result) = result {
                        match result {
                            RationaleResult::Same(_) | RationaleResult::Transform(_) => {
                                count += 1;
                            }
                            RationaleResult::None => continue,
                        }
                    } else {
                        return Err(FunctionError::InvalidInput);
                    }

                    match expected_count
                        .evaluate(Arc::new(Mutex::new(count.into())), &Default::default())
                        .await
                    {
                        Ok(RationaleResult::Same(_) | RationaleResult::Transform(_)) => {
                            return Ok(RationaleResult::Same(input.clone()))
                        }
                        Ok(RationaleResult::None) => {
                            continue;
                        }
                        Err(_) => return Err(FunctionError::InvalidInput),
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
            type test-some = list::Some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99, 42]);

        let result = runtime.evaluate("test::test-some", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_matching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-some = list::Some<2, $(self > 50)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 42, 99, 1024]);

        let result = runtime.evaluate("test::test-some", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-some = list::Some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 99, 4, 2]);

        let result = runtime.evaluate("test::test-some", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_homogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-some = list::Some<2, $(self > 101)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, 2, 3, 4, 5, 105, 99, 4, 2]);

        let result = runtime.evaluate("test::test-some", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }

    #[actix_rt::test]
    async fn call_matching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-some = list::Some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 42, 2, 3, 4, 5, 42, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-some", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_matching_heterogenous_type() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-some = list::Some<2, $(self > 99.0)>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 4, 5, 42, 99, "Bob", 99.1, 105]);

        let result = runtime.evaluate("test::test-some", value).await;

        assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_heterogenous_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-some = list::Some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([1, "taco", true, 2, 3, 42, 4, 5, 99, "Bob", 99.1]);

        let result = runtime.evaluate("test::test-some", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }

    #[actix_rt::test]
    async fn call_nonmatching_empty() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-some = list::Some<2, 42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!([]);

        let result = runtime.evaluate("test::test-some", value).await;

        assert!(matches!(result, Ok(RationaleResult::None),))
    }
}
