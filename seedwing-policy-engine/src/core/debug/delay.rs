use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerType, Type, ValueType};
use crate::package::Package;
use crate::runtime::{EvaluationResult, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::thread::sleep;
use std::time::Duration;

const DOCUMENTATION: &str = include_str!("DelayMs.adoc");

const DELAY: &str = "delay";

#[derive(Debug)]
pub struct DelayMs;

impl Function for DelayMs {
    fn order(&self) -> u8 {
        192
    }
    fn parameters(&self) -> Vec<String> {
        vec![DELAY.into()]
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
            if let Some(delay) = bindings.get(DELAY) {
                if let Some(ValueType::Integer(val)) = delay.try_get_resolved_value() {
                    sleep(Duration::from_millis(val as u64))
                }
            }

            Ok(Output::None.into())
        })
    }
}
