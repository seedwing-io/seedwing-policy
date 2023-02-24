use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("not.adoc");

const PATTERN: &str = "pattern";

#[derive(Debug)]
pub struct Not;

impl Function for Not {
    fn order(&self) -> u8 {
        128
    }
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            println!("A");
            if let Some(pattern) = bindings.get(PATTERN) {
                println!("B");
                let result = pattern.evaluate(input, ctx, bindings, world).await?;
                println!("C {result:?}");
                if result.satisfied() {
                    println!("D");
                    return Ok((Output::None, vec![result]).into());
                } else {
                    println!("E");
                    return Ok(Output::Identity.into());
                }
            }

            println!("F");
            Ok(Output::None.into())
        })
    }
}

#[cfg(test)]
mod test {

    use crate::core::test::test_pattern;

    use serde_json::json;

    #[actix_rt::test]
    async fn call_not_matching() {
        let result = test_pattern(
            r#"
            ! "bob"
            "#,
            json!("bob"),
        )
        .await;

        assert!(!result.satisfied())
    }

    #[actix_rt::test]
    async fn call_matching() {
        let result = test_pattern(
            r#"
            ! "bob"
            "#,
            json!("jim"),
        )
        .await;

        assert!(result.satisfied())
    }
}
