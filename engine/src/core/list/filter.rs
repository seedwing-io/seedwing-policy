use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::rationale::Rationale;
use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("filter.adoc");
const PARAMETER: &str = "parameter";

#[derive(Debug)]
pub struct Filter;

impl Function for Filter {
    fn order(&self) -> u8 {
        128
    }

    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![PARAMETER.to_string()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(binding) = bindings.get(PARAMETER) {
                match input.as_ref() {
                    RuntimeValue::List(inputs) => {
                        let mut result = Vec::new();
                        for input in inputs.iter() {
                            let eval = binding
                                .evaluate(input.clone(), ctx.push()?, bindings, world)
                                .await?;
                            match eval.severity() {
                                Severity::Error => {
                                    // skip
                                }
                                _ => result.push(input.clone()),
                            }
                        }
                        Ok(Output::Transform(Arc::new(RuntimeValue::List(result))).into())
                    }
                    _ => Ok((Severity::Error, Rationale::NotAList).into()),
                }
            } else {
                let msg = "Unable to find filter function";
                Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{assert_satisfied, runtime::testutil::test_pattern};

    #[tokio::test]
    async fn list_filter() {
        let json = serde_json::json!([1, 2, "foo"]);
        let result = test_pattern(r#"list::filter<integer>"#, json).await;
        assert_satisfied!(&result);
        assert_eq!(result.output().as_json(), serde_json::json!([1, 2]));
    }
}
