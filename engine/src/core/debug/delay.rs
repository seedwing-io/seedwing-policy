use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::{
    lir::{Bindings, ValuePattern},
    PatternMeta, Severity,
};
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

const DOCUMENTATION: &str = include_str!("delay-ms.adoc");

const DELAY: &str = "delay";

#[derive(Debug)]
pub struct DelayMs;

impl Function for DelayMs {
    fn order(&self) -> u8 {
        60
    }
    fn parameters(&self) -> Vec<String> {
        vec![DELAY.into()]
    }

    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn call<'v>(
        &'v self,
        _input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(delay) = bindings.get(DELAY) {
                if let Some(ValuePattern::Integer(val)) = delay.try_get_resolved_value() {
                    sleep(Duration::from_millis(val as u64))
                }
            }

            Ok(Severity::None.into())
        })
    }
}
