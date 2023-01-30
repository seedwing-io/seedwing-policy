use crate::core::{Function, FunctionEvaluationResult, json};
use crate::lang::lir::{Bindings, EvalContext};
use crate::package::Package;
use crate::runtime::{Output, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::{RationaleResult, RuntimeValue};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::Arc;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["json"]));
    pkg.register_function("JSON".into(), JSON);
    pkg
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct JSON;

const DOCUMENTATION: &str = include_str!("JSON.adoc");

impl Function for JSON {
    fn order(&self) -> u8 {
        128
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let input = (*input).borrow();
            if let Some(inner) = input.try_get_string() {
                let json_value: Result<serde_json::Value, _> =
                    serde_json::from_slice(inner.as_bytes());
                if let Ok(json_value) = json_value {
                    Ok(Output::Transform(Rc::new(json_value.into())).into())
                } else {
                    Ok(Output::None.into())
                }
            } else {
                Ok(Output::None.into())
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
    async fn call_parseable() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-json = json::JSON
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!(
            {
                "name": "Bob"
            }
        );

        let value = serde_json::to_string(&value).unwrap();

        let result = runtime.evaluate("test::test-json", value, EvalContext::default()).await;

        assert!(result.unwrap().satisfied())
        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn call_nonparseable() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-json = json::JSON
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = r#"
            I am not any valid JSON, dude, no, yes, true, false, ] {
        "#;

        let result = runtime.evaluate("test::test-json", value, EvalContext::default()).await;

        //assert!(matches!(result, Ok(RationaleResult::None),))
        assert!(!result.unwrap().satisfied())
    }
}
