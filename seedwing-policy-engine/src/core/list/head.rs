use super::COUNT;
use crate::core::list::split_fill;
use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, Type};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::{Object, RuntimeValue};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("Head.adoc");

#[derive(Debug)]
pub struct Head;

impl Function for Head {
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn parameters(&self) -> Vec<String> {
        vec![COUNT.into()]
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list().cloned() {
                let expected_count = bindings.get(COUNT);

                let (head, main) = split_fill(world, list.into_iter(), expected_count).await?;

                let mut result = Object::new();
                result.set("head", head);
                result.set("main", main);

                Ok(Output::Transform(Rc::new(result.into())).into())
            } else {
                Ok(Output::None.into())
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::super::test::*;
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvaluationResult;
    use serde_json::{json, Value};

    #[tokio::test]
    async fn call_matching_homogenous_literal() {
        let result = test_pattern(
            r#"list::Head<2>({
                head: [1, 2],
                main: [3, 4, 5, 41, 43, 99],
            })"#,
            json!([1, 2, 3, 4, 5, 41, 43, 99]),
        )
        .await;

        assert!(result.satisfied())
    }

    #[tokio::test]
    async fn call_matching_homogenous_literal_default() {
        let result = test_pattern(
            r#"list::Head({
                head: [1],
                main: [2, 3, 4, 5, 41, 43, 99],
            })"#,
            json!([1, 2, 3, 4, 5, 41, 43, 99]),
        )
        .await;

        assert!(result.satisfied())
    }

    #[tokio::test]
    async fn call_matching_homogenous_literal_no_main() {
        let result = test_pattern(
            r#"list::Head<2>({
                head: [1, 2],
                main: [],
            })"#,
            json!([1, 2]),
        )
        .await;

        assert!(result.satisfied())
    }

    #[tokio::test]
    async fn call_matching_homogenous_literal_less() {
        let result = test_pattern(
            r#"list::Head<2>({
                head: [1],
                main: [],
            })"#,
            json!([1]),
        )
        .await;

        assert!(result.satisfied())
    }

    #[tokio::test]
    async fn call_matching_homogenous_literal_empty() {
        let result = test_pattern(
            r#"list::Head<2>({
                head: [],
                main: [],
            })"#,
            json!([]),
        )
        .await;

        assert!(result.satisfied())
    }

    #[tokio::test]
    async fn call_matching_homogenous_literal_zero() {
        let result = test_pattern(
            r#"list::Head<0>({
                head: [],
                main: [1, 2, 3],
            })"#,
            json!([1, 2, 3]),
        )
        .await;

        assert!(result.satisfied())
    }
}
