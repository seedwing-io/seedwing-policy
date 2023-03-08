use crate::core::{Function, FunctionEvaluationResult};
use crate::data::DataSource;
use crate::lang::lir::{Bindings, InnerPattern, ValuePattern};
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct JSON;

const DOCUMENTATION: &str = include_str!("from.adoc");
const PATH: &str = "path";
const STEPS: &str = "steps";

#[derive(Debug)]
pub struct Lookup {
    data_sources: Arc<Vec<Arc<dyn DataSource>>>,
}

impl Lookup {
    pub fn new(data_sources: Vec<Arc<dyn DataSource>>) -> Self {
        Self {
            data_sources: Arc::new(data_sources),
        }
    }
}

impl Default for Lookup {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Function for Lookup {
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn parameters(&self) -> Vec<String> {
        vec![PATH.into(), STEPS.into()]
    }

    fn call<'v>(
        &'v self,
        _input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(val) = bindings.get(PATH) {
                if let Some(ValuePattern::String(path)) = val.try_get_resolved_value() {
                    for ds in &*self.data_sources {
                        if let Ok(Some(value)) = ds.get(path.clone()) {
                            if let Some(steps) = bindings.get(STEPS) {
                                if let InnerPattern::List(steps) = steps.inner() {
                                    let mut current = Arc::new(value);
                                    for step in steps {
                                        if let Some(step) = step.try_get_resolved_value() {
                                            if let ValuePattern::String(step) = step {
                                                if let Some(obj) = current.try_get_object() {
                                                    if let Some(next) = obj.get(step) {
                                                        current = next;
                                                    } else {
                                                        return Ok(Output::None.into());
                                                    }
                                                } else {
                                                    return Ok(Output::None.into());
                                                }
                                            } else {
                                                return Ok(Output::None.into());
                                            }
                                        } else {
                                            return Ok(Output::None.into());
                                        }
                                    }
                                    return Ok(Output::Transform(current).into());
                                } else {
                                    //todo!("support single non-list lookups")
                                    return Ok(Output::None.into());
                                }
                            } else {
                                return Ok(Output::None.into());
                            }
                        } else {
                            return Ok(Output::None.into());
                        }
                    }
                    return Ok(Output::None.into());
                } else {
                    return Ok(Output::None.into());
                }
            } else {
                return Ok(Output::None.into());
            }
        })
    }
}
