use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern, ValuePattern};
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("traverse.adoc");

const STEP: &str = "step";

#[derive(Debug)]
pub struct Traverse;

impl Function for Traverse {
    fn parameters(&self) -> Vec<String> {
        vec![STEP.into()]
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
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(step) = bindings.get(STEP) {
                if let InnerPattern::Const(ValuePattern::String(step)) = step.inner() {
                    if let Some(input) = input.try_get_object() {
                        if let Some(output) = input.get(step) {
                            return Ok(Output::Transform(output).into());
                        }
                    }
                }
            }

            Ok(Severity::Error.into())
        })
    }
}

#[cfg(test)]
mod test {
    use crate::value::RuntimeValue;
    use crate::{assert_satisfied, runtime::testutil::test_pattern};
    use serde_json::json;

    #[tokio::test]
    async fn traverse() {
        let json = json!({
            "person": { "name": "Fletch", "age": 48}
        });
        let result = test_pattern(r#"lang::traverse<"person">"#, json).await;
        assert_satisfied!(&result);
        assert!(result.output().is_object());
        let person = result.output().as_json();
        assert_eq!("Fletch", person.get("name").unwrap().as_str().unwrap());
        assert_eq!(RuntimeValue::Integer(48), person.get("age").unwrap().into());
    }
}
