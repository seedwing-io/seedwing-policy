use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern, ValuePattern};
use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("contains.adoc");
const SUBSTRING: &str = "substring";

#[derive(Debug)]
pub struct Contains;

impl Function for Contains {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![SUBSTRING.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(pattern) = bindings.get(SUBSTRING) {
                if let InnerPattern::Const(ValuePattern::String(substring)) = pattern.inner() {
                    if let Some(string) = input.try_get_string() {
                        return Ok(
                            Output::Transform(Arc::new(string.contains(substring).into())).into(),
                        );
                    }
                }
            }
            Ok(Output::Transform(Arc::new(false.into())).into())
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{assert_satisfied, runtime::testutil::test_pattern};
    use serde_json::json;

    #[tokio::test]
    async fn string_contains() {
        let result = test_pattern(
            r#"string::contains<"people">( $(self == true) )"#,
            json!("Some people like coffee."),
        )
        .await;
        assert_satisfied!(&result);
        assert!(result.output().try_get_boolean().unwrap());
    }

    #[tokio::test]
    async fn string_contains_no_substring() {
        let result = test_pattern(
            r#"string::contains( $(self == false) )"#,
            json!("anything old text here..."),
        )
        .await;
        assert_satisfied!(&result);
        assert!(!result.output().try_get_boolean().unwrap());
    }
}
