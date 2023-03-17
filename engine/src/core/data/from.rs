use crate::core::{Function, FunctionEvaluationResult};
use crate::data::DataSource;
use crate::lang::lir::{Bindings, ValuePattern};
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct JSON;

const DOCUMENTATION: &str = include_str!("from.adoc");
const PATH: &str = "path";

#[derive(Debug)]
pub struct From {
    data_sources: Arc<Vec<Arc<dyn DataSource>>>,
}

impl From {
    pub fn new(data_sources: Vec<Arc<dyn DataSource>>) -> Self {
        Self {
            data_sources: Arc::new(data_sources),
        }
    }
}

impl Default for From {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Function for From {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: Some(DOCUMENTATION.into()),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![PATH.into()]
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
                            return Ok(Output::Transform(Arc::new(value)).into());
                        }
                    }
                }
            }

            Ok(Output::None.into())
        })
    }
}
