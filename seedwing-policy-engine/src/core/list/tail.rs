use super::COUNT;
use crate::core::list::split_fill;
use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::{Object, RuntimeValue};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("tail.adoc");

#[derive(Debug)]
pub struct Tail;

impl Function for Tail {
    fn order(&self) -> u8 {
        128
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn parameters(&self) -> Vec<String> {
        vec![COUNT.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list().cloned() {
                let expected_count = bindings.get(COUNT);
                let (mut tail, mut main) =
                    split_fill(ctx, world, list.into_iter().rev(), expected_count).await?;

                tail.reverse();
                main.reverse();

                let mut result = Object::new();
                result.set("tail", tail);
                result.set("main", main);

                Ok(Output::Transform(Arc::new(result.into())).into())
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
            r#"list::tail<2>({
                tail: [43, 99],
                main: [1, 2, 3, 4, 5, 41],
            })"#,
            json!([1, 2, 3, 4, 5, 41, 43, 99]),
        )
        .await;

        assert!(result.satisfied())
    }

    #[tokio::test]
    async fn call_matching_homogenous_default() {
        let result = test_pattern(
            r#"list::tail({
                tail: [99],
                main: [1, 2, 3, 4, 5, 41, 43],
            })"#,
            json!([1, 2, 3, 4, 5, 41, 43, 99]),
        )
        .await;

        assert!(result.satisfied())
    }

    #[tokio::test]
    async fn call_matching_homogenous_literal_no_main() {
        let result = test_pattern(
            r#"list::tail<2>({
                tail: [1, 2],
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
            r#"list::tail<2>({
                tail: [1],
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
            r#"list::tail<2>({
                tail: [],
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
            r#"list::tail<0>({
                tail: [],
                main: [1, 2, 3],
            })"#,
            json!([1, 2, 3]),
        )
        .await;

        assert!(result.satisfied())
    }
}
