use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern};
use crate::lang::{Severity, ValuePattern};
use crate::runtime::rationale::Rationale;
use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("slice.adoc");
const START: &str = "start";
const END: &str = "end";

#[derive(Debug)]
pub struct Slice;

impl Function for Slice {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![START.into(), END.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Option::Some(list) = input.try_get_list() {
                let start = match get_parameter(START, bindings) {
                    Ok(value) => value,
                    Err(msg) => {
                        return invalid_arg(msg);
                    }
                };
                let end = match get_parameter(END, bindings) {
                    Ok(value) => value,
                    Err(msg) => {
                        return invalid_arg(msg);
                    }
                };
                if start > end {
                    return invalid_arg("start index cannot be greater than end index");
                }
                let s = &list[start..end];
                return Ok(Output::Transform(Arc::new(RuntimeValue::List(s.to_vec()))).into());
            }
            Ok(Output::Transform(Arc::new(RuntimeValue::List(Vec::new()))).into())
        })
    }
}

fn get_parameter(param: &str, bindings: &Bindings) -> Result<usize, String> {
    match bindings.get(param) {
        Some(pattern) => match pattern.inner() {
            InnerPattern::Const(ValuePattern::String(value)) => value
                .parse::<usize>()
                .map_err(|_| format!("invalid {param} index specified")),
            InnerPattern::Const(ValuePattern::Integer(value)) => Ok(*value as usize),
            _ => Err(format!("invalid {param} index specified")),
        },
        None => Err(format!("invalid type for {param} index")),
    }
}

fn invalid_arg(msg: impl Into<Arc<str>>) -> Result<FunctionEvaluationResult, RuntimeError> {
    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvalContext;
    use crate::{assert_not_satisfied, assert_satisfied};
    use serde_json::json;

    #[tokio::test]
    async fn list_slice() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern sl = list::slice<2, 4>
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::sl", json!([1, 2, 3, 4, 5]), EvalContext::default())
            .await
            .unwrap();
        assert_satisfied!(&result);

        let output = result.output();
        let list = output.try_get_list().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(3))));
        assert!(list.contains(&Arc::new(RuntimeValue::Integer(4))));
    }

    #[tokio::test]
    async fn list_slice_invalid_start_index() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern sl = list::slice<"x", 4>
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::sl", json!([1, 2, 3, 4, 5]), EvalContext::default())
            .await
            .unwrap();
        assert_not_satisfied!(&result);
        if let Rationale::Function {
            severity: _,
            rationale: out,
            supporting: _,
        } = result.rationale()
        {
            if let Rationale::InvalidArgument(msg) = &**(out.as_ref().unwrap()) {
                assert_eq!(msg, "invalid start index specified")
            }
        }
    }

    #[tokio::test]
    async fn list_slice_invalid_end_index() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern sl = list::slice<2, "x">
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::sl", json!([1, 2, 3, 4, 5]), EvalContext::default())
            .await
            .unwrap();
        assert_not_satisfied!(&result);
        if let Rationale::Function {
            severity: _,
            rationale: out,
            supporting: _,
        } = result.rationale()
        {
            if let Rationale::InvalidArgument(msg) = &**(out.as_ref().unwrap()) {
                assert_eq!(msg, "invalid end index specified");
            }
        }
    }

    #[tokio::test]
    async fn list_slice_invalid_index() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern sl = list::slice<4, 2>
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::sl", json!([1, 2, 3, 4, 5]), EvalContext::default())
            .await
            .unwrap();
        assert_not_satisfied!(&result);
        if let Rationale::Function {
            severity: _,
            rationale: out,
            supporting: _,
        } = result.rationale()
        {
            if let Rationale::InvalidArgument(msg) = &**(out.as_ref().unwrap()) {
                assert_eq!(msg, "start index cannot be greater than end index")
            }
        }
    }
}
