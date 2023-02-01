use crate::lang::hir::Type;
use crate::lang::lir::{Bindings, EvalContext, EvalTrace};
use crate::runtime::{EvaluationResult, Output, RuntimeError, World};
use crate::value::{RationaleResult, RuntimeValue};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use crate::runtime::rationale::Rationale;

pub mod base64;
pub mod cyclonedx;
pub mod data;
#[cfg(feature = "debug")]
pub mod debug;
pub mod iso;
pub mod json;
pub mod kafka;
pub mod lang;
pub mod list;
pub mod maven;
pub mod net;
pub mod pem;
#[cfg(feature = "sigstore")]
pub mod sigstore;
pub mod spdx;
pub mod string;
pub mod timestamp;
pub mod uri;
pub mod vex;
pub mod x509;

#[derive(Debug)]
pub struct FunctionEvaluationResult {
    function_output: Output,
    function_rationale: Option<Rationale>,
    supporting: Vec<EvaluationResult>,
}

impl FunctionEvaluationResult {
    pub fn output(&self) -> Output {
        self.function_output.clone()
    }

    pub fn rationale(&self) -> Option<Rationale> {
        self.function_rationale.clone()
    }

    pub fn supporting(&self) -> Vec<EvaluationResult> {
        self.supporting.clone()
    }
}

impl From<Output> for FunctionEvaluationResult {
    fn from(function_output: Output) -> Self {
        Self {
            function_output,
            function_rationale: None,
            supporting: vec![]
        }
    }
}

impl From<(Output, Vec<EvaluationResult>)> for FunctionEvaluationResult {
    fn from((function_output, supporting): (Output, Vec<EvaluationResult>)) -> Self {
        Self {
            function_output,
            function_rationale: None,
            supporting
        }
    }
}

impl From<(Output, Rationale)> for FunctionEvaluationResult {
    fn from((function_output, function_rationale): (Output, Rationale)) -> Self {
        Self {
            function_output,
            function_rationale: Some(function_rationale),
            supporting: vec![],
        }
    }
}

pub trait Function: Sync + Send + Debug {
    /// A number between 0 and u8::MAX indicating the evaluation order.
    ///
    /// 0 means the function is likely to be fast, 255 means likely to be slow.
    fn order(&self) -> u8;

    fn documentation(&self) -> Option<String> {
        None
    }

    fn parameters(&self) -> Vec<String> {
        Default::default()
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output=Result<FunctionEvaluationResult, RuntimeError>> + 'v>>;
}

/// A synchronous version of [`Function`].
pub trait SyncFunction: Sync + Send + Debug {
    /// A number between 0 and u8::MAX indicating the evaluation order.
    ///
    /// 0 means the function is likely to be fast, 255 means likely to be slow.
    fn order(&self) -> u8;

    fn documentation(&self) -> Option<String> {
        None
    }

    fn parameters(&self) -> Vec<String> {
        Default::default()
    }

    fn call(
        &self,
        input: Rc<RuntimeValue>,
        ctx: &mut EvalContext,
        bindings: &Bindings,
        world: &World,
    ) -> Result<FunctionEvaluationResult, RuntimeError>;
}

impl<F> Function for F
where
    F: SyncFunction,
{
    fn order(&self) -> u8 {
        SyncFunction::order(self)
    }

    fn documentation(&self) -> Option<String> {
        SyncFunction::documentation(self)
    }

    fn parameters(&self) -> Vec<String> {
        SyncFunction::parameters(self)
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async { SyncFunction::call(self, input, ctx, bindings, world) })
    }
}

#[cfg(test)]
mod test {
    use crate::lang::builder::Builder;
    use crate::lang::lir::EvalContext;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvaluationResult;
    use serde_json::{json, Value};

    pub(crate) async fn test_pattern(pattern: &str, value: Value) -> EvaluationResult {
        let src = format!("pattern test-pattern = {pattern}");
        println!("{src}");
        let src = Ephemeral::new("test", src);

        let mut builder = Builder::new();
        builder.build(src.iter()).unwrap();
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::test-pattern", value, EvalContext::default())
            .await;

        result.unwrap()
    }
}
