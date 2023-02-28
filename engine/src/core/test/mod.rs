use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::package::Package;
use crate::runtime::PackagePath;
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["test"]));
    pkg.register_function("satisfies".into(), Satisfies);
    pkg
}

const PATTERN: &str = "pattern";
const INPUT: &str = "input";

const DOCUMENTATION: &str = include_str!("satisfies.adoc");

#[derive(Debug)]
pub struct Satisfies;

impl Function for Satisfies {
    fn order(&self) -> u8 {
        128
    }
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into(), INPUT.into()]
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        _input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            match (bindings.get(PATTERN), bindings.get(INPUT)) {
                (Some(pattern), Some(input)) => {
                    if let Some(value) = input.try_get_resolved_value() {
                        let result = pattern
                            .evaluate(Arc::new(RuntimeValue::from(&value)), ctx, bindings, world)
                            .await?;

                        if result.satisfied() {
                            return Ok((Output::Identity, vec![result]).into());
                        } else {
                            Ok(Output::None.into())
                        }
                    } else {
                        Ok(Output::None.into())
                    }
                }
                _ => Ok(Output::None.into()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::builder::Builder as PolicyBuilder;
    use crate::runtime::sources::Ephemeral;

    #[actix_rt::test]
    async fn satisfies_assertion() {
        let policy = include_str!("mypolicy.dog");
        let tests = include_str!("mypolicy_test.dog");

        let policy = Ephemeral::new("mypolicy", policy);
        let tests = Ephemeral::new("mypolicy_test", tests);
        let mut builder = PolicyBuilder::new();

        builder.build(policy.iter());
        builder.build(tests.iter());

        let world = builder.finish().await.unwrap();
        assert!(world
            .evaluate("mypolicy_test::test1", "", EvalContext::default())
            .await
            .unwrap()
            .satisfied());
        assert!(world
            .evaluate("mypolicy_test::test2", "", EvalContext::default())
            .await
            .unwrap()
            .satisfied());
    }
}
