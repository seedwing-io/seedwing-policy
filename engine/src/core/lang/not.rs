use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{ExecutionContext, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("not.adoc");

const PATTERN: &str = "pattern";

#[derive(Debug)]
pub struct Not;

impl Function for Not {
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(pattern) = bindings.get(PATTERN) {
                let result = pattern
                    .evaluate(input, ctx.push()?, bindings, world)
                    .await?;
                let severity = match result.severity() {
                    Severity::Error => Severity::None,
                    _ => Severity::Error,
                };
                Ok((severity, vec![result]).into())
            } else {
                Ok(Severity::Error.into())
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::runtime::testutil::test_pattern;
    use crate::{assert_not_satisfied, assert_satisfied};
    use serde_json::json;

    #[tokio::test]
    async fn call_not_matching() {
        let result = test_pattern(
            r#"
            ! "bob"
            "#,
            json!("bob"),
        )
        .await;

        assert_not_satisfied!(result);
    }

    #[tokio::test]
    async fn call_matching() {
        let result = test_pattern(
            r#"
            ! "bob"
            "#,
            json!("jim"),
        )
        .await;

        assert_satisfied!(result);
    }
}
