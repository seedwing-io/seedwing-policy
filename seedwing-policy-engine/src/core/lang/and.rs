use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerType};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

const DOCUMENTATION: &str = include_str!("And.adoc");

const TERMS: &str = "terms";

#[derive(Debug)]
pub struct And;

impl Function for And {
    fn parameters(&self) -> Vec<String> {
        vec![TERMS.into()]
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(terms) = bindings.get(TERMS) {
                if let InnerType::List(terms) = terms.inner() {
                    let mut satisified = true;
                    let mut rationale = Vec::new();
                    for term in terms {
                        let result = term.evaluate(input.clone(), bindings, world).await?;
                        if !result.satisfied() {
                            satisified = false
                        }
                        rationale.push(result)
                    }

                    if satisified {
                        return Ok((Output::Identity, rationale).into());
                    } else {
                        return Ok((Output::None, rationale).into());
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
    async fn call_matching_both_arms() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern left = {
              first_name: "bob",
            }

            pattern right = {
              last_name: "mcw",
            }
            pattern test-and = left && right
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate(
                "test::test-and",
                json!(
                    {
                        "first_name": "bob",
                        "last_name": "mcw"
                    }
                ),
            )
            .await;
        assert!(result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_matching_only_left_arm() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern left = {
              first_name: "bob",
            }

            pattern right = {
              last_name: "mcw",
            }
            pattern test-and = left && right
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate(
                "test::test-and",
                json!(
                    {
                        "first_name": "bob"
                    }
                ),
            )
            .await;
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_matching_only_right_arm() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern left = {
              first_name: "bob",
            }

            pattern right = {
              last_name: "mcw",
            }
            pattern test-and = left && right
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate(
                "test::test-and",
                json!(
                    {
                        "last_name": "mcw"
                    }
                ),
            )
            .await;
        assert!(!result.unwrap().satisfied())
    }

    #[actix_rt::test]
    async fn call_matching_no_arms() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern left = {
              first_name: "bob",
            }

            pattern right = {
              last_name: "mcw",
            }
            pattern test-and = left && right
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate(
                "test::test-and",
                json!(
                    {
                        "first_name": "jim",
                        "last_name": "crossley"
                    }
                ),
            )
            .await;
        assert!(!result.unwrap().satisfied())
    }
}