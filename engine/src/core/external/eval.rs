use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::lang::ValuePattern;
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("eval.adoc");

const URL: &str = "url";

#[derive(Debug)]
pub struct Eval;

impl Function for Eval {
    fn order(&self) -> u8 {
        192
    }

    fn parameters(&self) -> Vec<String> {
        vec![URL.into()]
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(url) = bindings.get(URL) {
                if let Some(ValuePattern::String(url)) = url.try_get_resolved_value() {
                    let ext_input = input.as_json();
                    if let Ok(_ext_input) = serde_json::to_string(&ext_input) {
                        let client = reqwest::Client::new();
                        let res = client
                            .post(url)
                            .body("the exact body that is sent")
                            .send()
                            .await;

                        if let Ok(res) = res {
                            if res.status() == 200 {
                                // identity or transform
                                if let Some(len) = res.content_length() {
                                    if len > 0 {
                                        if let Ok(bytes) = res.bytes().await {
                                            let output: Result<Value, _> =
                                                serde_json::from_slice(&bytes);
                                            if let Ok(output) = output {
                                                let output: RuntimeValue = output.into();
                                                if *input.as_ref() == output {
                                                    Ok(Output::Identity.into())
                                                } else {
                                                    Ok(Output::Transform(Arc::new(output)).into())
                                                }
                                            } else {
                                                Ok(Output::Identity.into())
                                            }
                                        } else {
                                            Ok(Output::Identity.into())
                                        }
                                    } else {
                                        Ok(Output::Identity.into())
                                    }
                                } else {
                                    Ok(Output::Identity.into())
                                }
                            } else {
                                Ok(Output::None.into())
                            }
                        } else {
                            Ok(Output::None.into())
                        }
                    } else {
                        Ok(Output::None.into())
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
