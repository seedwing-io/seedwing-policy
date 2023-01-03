use crate::core::{json, Function, FunctionError};
use crate::lang::lir::Bindings;
use crate::lang::PackagePath;
use crate::package::Package;
use crate::value::Value;
use async_mutex::Mutex;
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

#[derive(Debug)]
pub struct JSON;

const DOCUMENTATION: &str = include_str!("JSON.adoc");

impl Function for JSON {
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: &'v Value,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<Value, FunctionError>> + 'v>> {
        Box::pin(async move {
            if let Some(inner) = input.try_get_string() {
                let json_value: Result<serde_json::Value, _> =
                    serde_json::from_slice(inner.as_bytes());
                if let Ok(json_value) = json_value {
                    Ok(json_value.into())
                } else {
                    Err(FunctionError::Other("unable to decode as json".into()))
                }
            } else {
                Err(FunctionError::InvalidInput)
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
            type test-json = json::JSON
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

        let result = runtime.evaluate("test::test-json", value).await;

        assert!(matches!(result, Ok(Some(_)),))
    }

    #[actix_rt::test]
    async fn call_nonparseable() {
        let src = Ephemeral::new(
            "test",
            r#"
            type test-json = json::JSON
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = r#"
            I am not any valid JSON, dude, no, yes, true, false, ] {
        "#;

        let result = runtime.evaluate("test::test-json", value).await;

        assert!(matches!(result, Ok(None),))
    }
}
