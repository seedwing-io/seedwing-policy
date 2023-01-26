use crate::lang::hir::Type;
use crate::lang::lir::Bindings;
use crate::runtime::{EvaluationResult, Output, RuntimeError, World};
use crate::value::{RationaleResult, RuntimeValue};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

pub mod base64;
pub mod cyclonedx;
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
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>>;
}

#[cfg(test)]
mod test {
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvaluationResult;
    use serde_json::{json, Value};

    pub(crate) async fn test_pattern(pattern: &str, value: Value) -> EvaluationResult {
        let src = format!("pattern test-pattern = {pattern}");
        println!("{}", src);
        let src = Ephemeral::new("test", src);

        let mut builder = Builder::new();
        builder.build(src.iter()).unwrap();
        let runtime = builder.finish().await.unwrap();
        let result = runtime.evaluate("test::test-pattern", value).await;

        result.unwrap()
    }
}
