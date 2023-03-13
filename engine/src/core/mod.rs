use crate::lang::lir::Bindings;
use crate::runtime::rationale::Rationale;
use crate::runtime::{
    EvalContext, EvaluationResult, Output, Pattern, PatternName, RuntimeError, World,
};
use crate::value::RuntimeValue;

use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

pub mod base64;
pub mod config;
pub mod csaf;
pub mod cyclonedx;
pub mod data;
#[cfg(feature = "debug")]
pub mod debug;
pub mod external;
pub mod guac;
pub mod intoto;
pub mod iso;
pub mod jsf;
pub mod json;
pub mod kafka;
pub mod lang;
pub mod list;
pub mod maven;
pub mod net;
pub mod openvex;
pub mod osv;
pub mod pem;
pub mod rhsa;
#[cfg(feature = "showcase")]
pub mod showcase;
#[cfg(feature = "sigstore")]
pub mod sigstore;
pub mod slsa;
pub mod spdx;
pub mod string;
pub mod timestamp;
pub mod uri;
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
            supporting: vec![],
        }
    }
}

impl From<(Output, Vec<EvaluationResult>)> for FunctionEvaluationResult {
    fn from((function_output, supporting): (Output, Vec<EvaluationResult>)) -> Self {
        Self {
            function_output,
            function_rationale: None,
            supporting,
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

#[derive(Debug)]
pub enum FunctionInput {
    Anything,
    String,
    Boolean,
    Integer,
    Decimal,
    Pattern(FunctionInputPattern),
}

#[derive(Debug)]
pub enum FunctionInputPattern {
    Just(PatternName),
    And(Vec<FunctionInput>),
    Or(Vec<FunctionInput>),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Example {
    /// A distinct identifier
    pub name: String,
    /// An optional summary override, should override the use of the name when presenting the user.
    pub summary: Option<String>,
    /// An optional detailed description
    pub description: Option<String>,
    /// The value of the example
    pub value: serde_json::Value,
}

pub trait Function: Sync + Send + Debug {
    fn input(&self, _bindings: &[Arc<Pattern>]) -> FunctionInput {
        FunctionInput::Anything
    }

    /// A number between 0 and u8::MAX indicating the evaluation order.
    ///
    /// 0 means the function is likely to be fast, 255 means likely to be slow.
    /// Guidance:
    /// 0 - 11 - Fast non-async lookup/conversion code
    /// 12 - 40 - More complex non-async code
    /// 40 - 120 - Async code
    /// 120 - 255 - Code that uses network or disk.
    fn order(&self) -> u8 {
        7
    }

    fn metadata(&self) -> PatternMeta {
        Default::default()
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }

    fn parameters(&self) -> Vec<String> {
        Default::default()
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>>;
}

/// A blocking version of [`Function`].
pub trait BlockingFunction: Sync + Send + Debug {
    /// A number between 0 and u8::MAX indicating the evaluation order.
    ///
    /// 0 means the function is likely to be fast, 255 means likely to be slow.
    /// Guidance:
    /// 0 - 11 - Fast non-async lookup/conversion code
    /// 12 - 40 - More complex non-async code
    /// 40 - 120 - Async code
    /// 120 - 255 - Code that uses network or disk.
    fn order(&self) -> u8 {
        7
    }

    fn metadata(&self) -> PatternMeta {
        Default::default()
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }

    fn parameters(&self) -> Vec<String> {
        Default::default()
    }

    fn call(
        &self,
        input: Arc<RuntimeValue>,
        ctx: &EvalContext,
        bindings: &Bindings,
        world: &World,
    ) -> Result<FunctionEvaluationResult, RuntimeError>;
}

impl<F> Function for F
where
    F: BlockingFunction,
{
    fn order(&self) -> u8 {
        BlockingFunction::order(self)
    }

    fn metadata(&self) -> PatternMeta {
        BlockingFunction::metadata(self)
    }

    fn examples(&self) -> Vec<Example> {
        BlockingFunction::examples(self)
    }

    fn parameters(&self) -> Vec<String> {
        BlockingFunction::parameters(self)
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async { BlockingFunction::call(self, input, ctx, bindings, world) })
    }
}
