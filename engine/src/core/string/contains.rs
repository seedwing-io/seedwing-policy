use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern, ValuePattern};
use crate::runtime::{EvalContext, Output, RuntimeError, World};
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
        _ctx: &'v EvalContext,
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
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use serde_json::json;

    #[actix_rt::test]
    async fn string_contains() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern has_str = string::contains<"people">( $(self == true) )
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate(
                "test::has_str",
                json!("Some people like coffee."),
                EvalContext::default(),
            )
            .await;
        assert!(result.as_ref().unwrap().satisfied());
        assert!(result
            .as_ref()
            .unwrap()
            .output()
            .unwrap()
            .try_get_boolean()
            .unwrap());
    }

    #[actix_rt::test]
    async fn string_contains_no_substring() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern no_substring = string::contains( $(self == false) )
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate(
                "test::no_substring",
                json!("anything old text here..."),
                EvalContext::default(),
            )
            .await;
        assert!(result.as_ref().unwrap().satisfied());
        assert!(!result
            .as_ref()
            .unwrap()
            .output()
            .unwrap()
            .try_get_boolean()
            .unwrap());
    }
}
