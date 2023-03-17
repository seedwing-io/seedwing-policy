use crate::core::{BlockingFunction, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::lang::PatternMeta;
use crate::package::Package;
use crate::runtime::{EvalContext, Output, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::RuntimeValue;
use std::sync::Arc;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["showcase"]));
    pkg.register_function("builtin".into(), BuiltIn);
    pkg.register_source("".into(), include_str!("meta.dog"));
    pkg
}

#[derive(Debug)]
pub struct BuiltIn;

const DOCUMENTATION: &str = include_str!("builtin.adoc");

impl BlockingFunction for BuiltIn {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: Some(DOCUMENTATION.into()),
            ..Default::default()
        }
    }

    fn call(
        &self,
        _input: Arc<RuntimeValue>,
        _ctx: &EvalContext,
        _bindings: &Bindings,
        _world: &World,
    ) -> Result<FunctionEvaluationResult, RuntimeError> {
        // does nothing
        Ok(Output::Identity.into())
    }
}
