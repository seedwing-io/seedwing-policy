use crate::core::{Function, FunctionEvaluationResult, FunctionInput};
use crate::lang::lir::Bindings;
use crate::runtime::{EvalContext, Output, Pattern, RuntimeError, World};
use crate::value::Object;
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct GAV;
const DOCUMENTATION: &str = include_str!("GAV.adoc");

impl Function for GAV {
    fn input(&self, _bindings: &[Arc<Pattern>]) -> FunctionInput {
        FunctionInput::String
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
            if let Some(gav) = input.try_get_string() {
                let parts: Vec<&str> = gav.split(':').collect();
                if parts.len() >= 3 && parts.len() <= 5 {
                    let group_id = parts[0];
                    let artifact_id = parts[1];
                    let version = parts[2];
                    let packaging = if parts.len() >= 4 { parts[3] } else { "jar" };

                    let classifier = if parts.len() == 5 {
                        Some(parts[4])
                    } else {
                        None
                    };

                    let mut coordinates = Object::new();
                    coordinates.set::<&str, &str>("groupId", group_id);
                    coordinates.set::<&str, &str>("artifactId", artifact_id);
                    coordinates.set::<&str, &str>("version", version);
                    coordinates.set::<&str, &str>("packaging", packaging);
                    if let Some(classifier) = classifier {
                        coordinates.set::<&str, &str>("classifier", classifier);
                    }

                    Ok(Output::Transform(Arc::new(coordinates.into())).into())
                } else {
                    Ok(Severity::Error.into())
                }
            } else {
                Ok(Severity::Error.into())
            }
        })
    }
}
