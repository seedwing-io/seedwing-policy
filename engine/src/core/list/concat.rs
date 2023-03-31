use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern};
use crate::runtime::rationale::Rationale;
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("concat.adoc");
const LIST: &str = "list";

#[derive(Debug)]
pub struct Concat;

impl Function for Concat {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![LIST.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Option::Some(input_list) = input.try_get_list() {
                let list = match get_parameter(LIST, bindings) {
                    Ok(value) => value,
                    Err(msg) => {
                        return invalid_arg(msg);
                    }
                };
                let mut input_list = input_list.clone();
                input_list.append(&mut list.to_vec());
                return Ok(Self::output(input_list).into());
            }
            Ok(Self::output(Vec::new()).into())
        })
    }
}

impl Concat {
    fn output(list: Vec<Arc<RuntimeValue>>) -> Output {
        Output::Transform(Arc::new(RuntimeValue::List(list)))
    }
}

fn get_parameter(param: &str, bindings: &Bindings) -> Result<Vec<Arc<RuntimeValue>>, String> {
    match bindings.get(param) {
        Some(pattern) => match pattern.inner() {
            InnerPattern::List(list) => {
                let mut new_list: Vec<Arc<RuntimeValue>> = Vec::new();
                for item in list {
                    if let Some(value) = item.try_get_resolved_value() {
                        new_list.push(Arc::new(RuntimeValue::from(&value)));
                    }
                }
                Ok(new_list)
            }
            _ => Err(format!("invalid type specified for {param} parameter")),
        },
        None => Err(format!("invalid type specified for {param} parameter")),
    }
}

fn invalid_arg(msg: impl Into<String>) -> Result<FunctionEvaluationResult, RuntimeError> {
    Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{assert_not_satisfied, assert_satisfied, runtime::testutil::test_pattern};
    use serde_json::json;

    #[actix_rt::test]
    async fn list_concat() {
        let result = test_pattern(r#"list::concat<[4, 5, 6]>"#, json!([1, 2, 3])).await;
        assert_satisfied!(result);

        let output = result.output().unwrap();
        let list = output.try_get_list().unwrap();
        assert_eq!(list.len(), 6);
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(1))));
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(2))));
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(3))));
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(4))));
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(5))));
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(6))));
    }

    #[actix_rt::test]
    async fn list_append_empty_list() {
        let result = test_pattern(r#"list::append<[1, 2, 3]>"#, json!([])).await;
        assert_satisfied!(result);

        let output = result.output().unwrap();
        let list = output.try_get_list().unwrap();
        assert_eq!(list.len(), 3);
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(1))));
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(2))));
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(3))));
    }

    #[actix_rt::test]
    async fn list_concat_invalid_input() {
        let result = test_pattern(r#"list::concat<"some string">"#, json!([1, 2, 3])).await;
        assert_not_satisfied!(result);

        if let Rationale::Function(_, out, _) = result.rationale() {
            if let Rationale::InvalidArgument(msg) = &**(out.as_ref().unwrap()) {
                assert_eq!(msg, "invalid type specified for list parameter")
            }
        }
    }
}
