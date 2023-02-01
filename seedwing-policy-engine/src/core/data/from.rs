use crate::core::{Function, FunctionEvaluationResult};
use crate::data::DataSource;
use crate::lang::lir::{Bindings, EvalContext, ValueType};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct JSON;

const DOCUMENTATION: &str = include_str!("From.adoc");
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

impl Function for From {
    fn order(&self) -> u8 {
        128
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn parameters(&self) -> Vec<String> {
        vec![PATH.into()]
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(val) = bindings.get(PATH) {
                if let Some(ValueType::String(path)) = val.try_get_resolved_value() {
                    for ds in &*self.data_sources {
                        if let Ok(Some(value)) = ds.get(path.clone()) {
                            return Ok(Output::Transform(Rc::new(value)).into());
                        }
                    }
                }
            }

            Ok(Output::None.into())
        })
    }
}
