use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::World;
use crate::runtime::{EvalContext, Output, RuntimeError};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

use super::merge;
use crate::lang::PatternMeta;
use openvex::*;

#[derive(Debug)]
pub struct Merge;

const DOCUMENTATION: &str = include_str!("merge.adoc");

impl Function for Merge {
    fn order(&self) -> u8 {
        9
    }
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let RuntimeValue::List(items) = input.as_ref() {
                let mut vexes = Vec::new();
                for item in items.iter() {
                    if let Ok(vex) = serde_json::from_value::<OpenVex>(item.as_json()) {
                        vexes.push(vex);
                    }
                }
                let result = merge(vexes);
                let json: serde_json::Value = serde_json::to_value(result).unwrap();
                Ok(Output::Transform(Arc::new(json.into())).into())
            } else {
                Ok(Output::Transform(input).into())
            }
        })
    }
}
