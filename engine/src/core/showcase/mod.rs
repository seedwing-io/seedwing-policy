use crate::core::{BlockingFunction, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::lang::{PatternMeta, Severity};
use crate::package::Package;
use crate::runtime::{ExecutionContext, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::RuntimeValue;
use std::sync::Arc;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["showcase"]))
        .with_documentation("Showcase a few features of seedwing");
    pkg.register_function("builtin".into(), BuiltIn);
    pkg.register_source("".into(), include_str!("meta.dog"));
    pkg
}

/// Example for a built-in function, adding metadata.
#[derive(Debug)]
pub struct BuiltIn;

const DOCUMENTATION: &str = include_str!("builtin.adoc");

impl BlockingFunction for BuiltIn {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn call(
        &self,
        _input: Arc<RuntimeValue>,
        _ctx: ExecutionContext<'_>,
        _bindings: &Bindings,
        _world: &World,
    ) -> Result<FunctionEvaluationResult, RuntimeError> {
        // does nothing
        Ok(Severity::None.into())
    }
}
