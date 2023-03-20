use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::rationale::Rationale;
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
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
            documentation: Some(DOCUMENTATION.into()),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![PARAMETER.to_string()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(binding) = bindings.get(PARAMETER) {
                match input.as_ref() {
                    RuntimeValue::List(inputs) => {
                        let mut result = Vec::new();
                        for input in inputs.iter() {
                            match binding
                                .evaluate(input.clone(), ctx, bindings, world)
                                .await?
                                .raw_output()
                            {
                                Output::Identity => result.push(input.clone()),
                                Output::Transform(_) => result.push(input.clone()),
                                Output::None => {}
                            }
                        }
                        Ok(Output::Transform(Arc::new(RuntimeValue::List(result.clone()))).into())
                    }
                    _ => {
                        let msg = "Input is not a list";
                        Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
                    }
                }
            } else {
                let msg = "Unable to find filter function";
                Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
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
        assert_satisfied!(result);
        assert_eq!(
            result.output().unwrap().as_json(),
            serde_json::json!([1, 2])
        );
    }
}
