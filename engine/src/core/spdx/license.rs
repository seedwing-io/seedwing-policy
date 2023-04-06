use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use spdx;

use crate::lang::{PatternMeta, Severity};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("expr.adoc");

#[derive(Debug)]
pub struct Expression;

impl Function for Expression {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(value) = input.try_get_string() {
                if spdx::Expression::parse(value.as_str()).is_ok() {
                    return Ok(Output::Identity.into());
                }
            }
            Ok(Severity::Error.into())
        })
    }
}

#[cfg(test)]
mod test {

    use crate::runtime::testutil::test_pattern;
    use crate::{assert_not_satisfied, assert_satisfied};
    use serde_json::json;

    #[tokio::test]
    async fn valid_expression() {
        let result = test_pattern(r#"spdx::license-expr"#, json!("GPL-2.0-only")).await;
        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn invalid_expression() {
        let result = test_pattern(r#"spdx::license-expr"#, json!("Bogus")).await;
        assert_not_satisfied!(result);
    }
}
