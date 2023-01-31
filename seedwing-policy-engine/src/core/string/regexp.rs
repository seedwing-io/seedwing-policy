use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext, InnerType, ValueType};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use regex::Regex;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

const DOCUMENTATION: &str = include_str!("Regexp.adoc");
const REGEXP: &str = "regexp";

#[derive(Debug)]
pub struct Regexp;

impl Function for Regexp {
    fn order(&self) -> u8 {
        140
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn parameters(&self) -> Vec<String> {
        vec![REGEXP.into()]
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(regexp) = bindings.get(REGEXP) {
                if let Some(ValueType::String(regexp)) = regexp.try_get_resolved_value() {
                    if let Some(value) = input.try_get_string() {
                        if let Ok(regexp) = Regex::new(regexp.as_str()) {
                            if regexp.is_match(value.as_str()) {
                                return Ok(Output::Identity.into());
                            }
                        }
                    }
                }
            }
            Ok(Output::None.into())
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
    async fn call_matching_with_valid_param() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern re = string::Regexp<"bob.*mcwhirter">
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate(
                "test::re",
                json!("bob dude mcwhirter"),
                EvalContext::default(),
            )
            .await;
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_nonmatching_with_valid_param() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern re = string::Regexp<"bob.*mcwhirter">
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate(
                "test::re",
                json!("bob subgenius dobbs"),
                EvalContext::default(),
            )
            .await;
        assert!(!result.unwrap().satisfied());
    }

    #[actix_rt::test]
    async fn call_nonmatching_with_invalid_param() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern re = string::Regexp<42>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate(
                "test::re",
                json!("bob subgenius dobbs"),
                EvalContext::default(),
            )
            .await;
        assert!(!result.unwrap().satisfied());
    }
}
