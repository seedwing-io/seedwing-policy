use crate::lang::hir::Type;
use crate::lang::lir::Bindings;
use crate::runtime::{EvaluationResult, Output, RuntimeError};
use crate::value::{RationaleResult, RuntimeValue};
use std::cell::RefCell;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

pub mod base64;
pub mod json;
pub mod list;
pub mod pattern;
pub mod sigstore;
pub mod x509;

#[derive(Debug)]
pub struct FunctionEvaluationResult(Output, Vec<EvaluationResult>);

impl FunctionEvaluationResult {
    pub fn output(&self) -> Output {
        self.0.clone()
    }

    pub fn supporting(&self) -> Vec<EvaluationResult> {
        self.1.clone()
    }
}

impl From<Output> for FunctionEvaluationResult {
    fn from(output: Output) -> Self {
        Self(output, vec![])
    }
}

impl From<(Output, Vec<EvaluationResult>)> for FunctionEvaluationResult {
    fn from(inner: (Output, Vec<EvaluationResult>)) -> Self {
        Self(inner.0, inner.1)
    }
}

pub trait Function: Sync + Send + Debug {
    fn documentation(&self) -> Option<String> {
        None
    }

    fn parameters(&self) -> Vec<String> {
        Default::default()
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>>;
}
