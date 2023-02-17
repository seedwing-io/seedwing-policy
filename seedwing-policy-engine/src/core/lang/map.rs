use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::runtime::rationale::Rationale;
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("map.adoc");

const MAP_FN: &str = "map-fn";

#[derive(Debug)]
pub struct Map;

impl Function for Map {
    fn order(&self) -> u8 {
        128
    }
    fn parameters(&self) -> Vec<String> {
        vec![MAP_FN.into()]
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
            if let Some(map_fn) = bindings.get(MAP_FN) {
                match input.as_ref() {
                    RuntimeValue::List(inputs) => {
                        let mut result = Vec::new();
                        for input in inputs.iter() {
                            if let Some(value) = map_fn
                                .evaluate(input.clone(), ctx, bindings, world)
                                .await?
                                .output()
                            {
                                result.push(value);
                            } else {
                                let msg = "No output from map function";
                                return Ok(
                                    (Output::None, Rationale::InvalidArgument(msg.into())).into()
                                );
                            }
                        }
                        Ok(Output::Transform(Arc::new(RuntimeValue::List(result.clone()))).into())
                    }
                    _ => Ok(map_fn
                        .evaluate(input, ctx, bindings, world)
                        .await?
                        .raw_output()
                        .clone()
                        .into()),
                }
            } else {
                let msg = "Unable to lookup map function";
                Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test::test_pattern;
    use serde_json::json;

    #[tokio::test]
    async fn test_map_single_element() {
        let result = test_pattern(
            r#"lang::map<uri::purl>"#,
            RuntimeValue::String(
                "pkg:github/package-url/purl-spec@244fd47e07d1004#everybody/loves/dogs".to_string(),
            ),
        )
        .await;

        assert_eq!(
            result.output(),
            Some(Arc::new(
                json!({
                    "type": "github",
                    "namespace": "package-url",
                    "name": "purl-spec",
                    "version": "244fd47e07d1004",
                    "subpath": "everybody/loves/dogs",
                })
                .into()
            ))
        );
    }

    #[tokio::test]
    async fn test_map_list() {
        let result = test_pattern(
            r#"lang::map<uri::purl>"#,
            vec![
                RuntimeValue::String(
                    "pkg:github/package-url/purl-spec@244fd47e07d1004#everybody/loves/dogs"
                        .to_string(),
                ),
                RuntimeValue::String(
                    "pkg:github/package-url/purl-spec@244fd47e07d1004#everybody/loves/cats"
                        .to_string(),
                ),
            ],
        )
        .await;

        assert_eq!(
            result.output(),
            Some(Arc::new(
                json!([{
                    "type": "github",
                    "namespace": "package-url",
                    "name": "purl-spec",
                    "version": "244fd47e07d1004",
                    "subpath": "everybody/loves/dogs",
                }, {
                    "type": "github",
                    "namespace": "package-url",
                    "name": "purl-spec",
                    "version": "244fd47e07d1004",
                    "subpath": "everybody/loves/cats",
                }])
                .into()
            ))
        );
    }
}
