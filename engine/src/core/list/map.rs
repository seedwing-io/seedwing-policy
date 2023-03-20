use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, Pattern};
use crate::runtime::rationale::Rationale;
use crate::runtime::{response::reason, EvalContext, Output, RuntimeError, World};
use crate::value::{Object, RuntimeValue};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("map.adoc");

const MAP_FN: &str = "map-fn";

#[derive(Debug)]
pub struct Map;

impl Map {
    async fn eval_element(
        &self,
        map_fn: Arc<Pattern>,
        input: Arc<RuntimeValue>,
        ctx: &EvalContext,
        bindings: &Bindings,
        world: &World,
    ) -> Result<RuntimeValue, RuntimeError> {
        let mut object = Object::new();
        let result = map_fn.evaluate(input.clone(), ctx, bindings, world).await?;
        match result.raw_output() {
            Output::None => {
                object.set("reason", reason(result.rationale()));
            }
            Output::Identity => {
                object.set("value", input.as_json());
            }
            Output::Transform(value) => {
                object.set("value", value.as_json());
            }
        }
        Ok(RuntimeValue::Object(object))
    }
}

impl Function for Map {
    fn parameters(&self) -> Vec<String> {
        vec![MAP_FN.into()]
    }

    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: Some(DOCUMENTATION.into()),
            ..Default::default()
        }
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
                        let mut result: Vec<Arc<RuntimeValue>> = Vec::new();
                        for input in inputs.iter() {
                            let value = self
                                .eval_element(map_fn.clone(), input.clone(), ctx, bindings, world)
                                .await?;
                            result.push(value.into());
                        }
                        Ok(Output::Transform(Arc::new(RuntimeValue::List(result.clone()))).into())
                    }
                    _ => {
                        let msg = "Input is not a list";
                        Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
                    }
                }
            } else {
                let msg = "Unable to lookup map function";
                Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
            }
        })
    }
}

#[derive(Serialize)]
pub struct MapOutput {
    value: Option<Arc<RuntimeValue>>,
    reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_not_satisfied;
    use crate::runtime::testutil::test_pattern;
    use serde_json::json;

    #[tokio::test]
    async fn test_map_single_element() {
        let result = test_pattern(
            r#"list::map<uri::purl>"#,
            RuntimeValue::String(
                "pkg:github/package-url/purl-spec@244fd47e07d1004#everybody/loves/dogs".to_string(),
            ),
        )
        .await;

        assert_not_satisfied!(result);
    }

    #[tokio::test]
    async fn test_map_list() {
        let result = test_pattern(
            r#"list::map<uri::purl>"#,
            vec![
                RuntimeValue::String(
                    "pkg:github/package-url/purl-spec@244fd47e07d1004#everybody/loves/dogs"
                        .to_string(),
                ),
                RuntimeValue::String(
                    "pkg:github/package-url/purl-spec@244fd47e07d1004#everybody/loves/cats"
                        .to_string(),
                ),
                RuntimeValue::Integer(44),
            ],
        )
        .await;

        assert_eq!(
            result.output(),
            Some(Arc::new(
                json!([
                    {
                        "value": {
                            "type": "github",
                            "namespace": "package-url",
                            "name": "purl-spec",
                            "version": "244fd47e07d1004",
                            "subpath": "everybody/loves/dogs",
                        }
                    },
                    {
                        "value": {
                            "type": "github",
                            "namespace": "package-url",
                            "name": "purl-spec",
                            "version": "244fd47e07d1004",
                            "subpath": "everybody/loves/cats",
                        }
                    },
                    {
                        "reason": "invalid argument: input is neither a String nor an Object"
                    }
                ])
                .into()
            ))
        );
    }
}
