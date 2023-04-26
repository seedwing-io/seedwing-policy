use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern};
use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("contains-all.adoc");
const PARAMETER: &str = "parameter";

#[derive(Debug)]
pub struct ContainsAll;

impl Function for ContainsAll {
    fn order(&self) -> u8 {
        128
    }

    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![PARAMETER.to_string()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(binding) = bindings.get(PARAMETER) {
                if let InnerPattern::List(items) = binding.inner() {
                    if let Some(list) = input.try_get_list() {
                        // Iterate over the list items first because they are more likely to be long.
                        let mut matched: Vec<bool> = vec![false; items.len()];
                        for (idx, item) in items.iter().enumerate() {
                            for s in list {
                                let result = item
                                    .evaluate(s.clone(), ctx.push()?, bindings, world)
                                    .await?;
                                if result.severity() != Severity::Error {
                                    matched[idx] = true;
                                    break;
                                }
                            }
                            if matched.iter().filter(|v| **v).count() == matched.len() {
                                return Ok(Output::Identity.into());
                            }
                        }
                        return Ok(Severity::Error.into());
                    }
                }
            }
            Ok(Severity::Error.into())
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{assert_not_satisfied, assert_satisfied, runtime::testutil::test_pattern};

    #[tokio::test]
    async fn list_contains() {
        let json = serde_json::json!(["foo", "bar", "baz"]);
        let result = test_pattern(r#"list::contains-all<["foo", "bar"]>"#, json).await;
        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn list_contains_patterns() {
        let json = serde_json::json!(["foo", "bar", "baz"]);
        let result = test_pattern(r#"list::contains-all<[string, integer]>"#, json).await;
        assert_not_satisfied!(result);

        let json = serde_json::json!(["foo", "bar", 2]);
        let result = test_pattern(r#"list::contains-all<[string, integer]>"#, json).await;
        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn list_identical() {
        let json = serde_json::json!(["foo", "bar", "baz"]);
        let result = test_pattern(r#"list::contains-all<["foo", "bar", "baz"]>"#, json).await;
        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn list_not_contains() {
        let json = serde_json::json!(["foo", "baz"]);
        let result = test_pattern(r#"list::contains-all<["foo", "bar"]>"#, json).await;
        assert_not_satisfied!(result);
    }
}
