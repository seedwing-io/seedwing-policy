use crate::core::{json, Function, FunctionEvaluationResult};
use crate::lang::lir::Type;
use crate::lang::lir::{Bindings, InnerType};
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
    let mut pkg = Package::new(PackagePath::from_parts(vec!["pattern"]));
    pkg.register_function("Set".into(), Set);
    pkg
}

const PATTERN: &str = "pattern";

#[derive(Debug)]
pub struct Set;

const DOCUMENTATION: &str = include_str!("Set.adoc");

impl Function for Set {
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(pattern) = bindings.get(PATTERN) {
                if let InnerType::List(terms) = pattern.inner() {
                    let set = Type::new(None, None, Vec::default(), InnerType::Join(terms.clone()));
                    let result = Arc::new(set).evaluate(input, bindings, world).await?;
                    if result.satisfied() {
                        Ok((Output::Identity, vec![result]).into())
                    } else {
                        Ok((Output::None, vec![result]).into())
                    }
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
    async fn call() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern names = [ "bob", "ulf", "jim" ]
            pattern people = pattern::Set<names>
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!("Bob");

        let result = runtime.evaluate("test::people", json!("bob")).await;
        assert!(result.unwrap().satisfied());

        let result = runtime.evaluate("test::people", json!("jim")).await;
        assert!(result.unwrap().satisfied());

        let result = runtime.evaluate("test::people", json!("Mr. Ulf")).await;
        assert!(!result.unwrap().satisfied());
    }
}
