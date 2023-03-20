use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::lang::PatternMeta;
use crate::lang::ValuePattern;
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("config.adoc");

const KEY: &str = "key";

#[derive(Debug)]
pub struct Of;

impl Function for Of {
    fn order(&self) -> u8 {
        192
    }

    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![KEY.into()]
    }

    fn call<'v>(
        &'v self,
        _input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(key) = bindings.get(KEY) {
                if let Some(ValuePattern::String(key)) = key.try_get_resolved_value() {
                    if let Some(value) = ctx.config().get(&key) {
                        Ok(Output::Transform(Arc::new(value.into())).into())
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
