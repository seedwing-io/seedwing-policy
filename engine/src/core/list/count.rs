use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("count.adoc");

#[derive(Debug)]
pub struct Count;

impl Function for Count {
    fn order(&self) -> u8 {
        128
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
        _ctx: ExecutionContext<'v>,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list() {
                Ok(Output::Transform(Arc::new(list.len().into())).into())
            } else {
                Ok(Output::Transform(Arc::new(0.into())).into())
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{assert_satisfied, runtime::testutil::test_pattern};
    use serde_json::json;

    #[tokio::test]
    async fn list_count() {
        let result = test_pattern(r#"list::count( $(self == 4) )"#, json!([1, 2, 3, 4])).await;
        assert_satisfied!(&result);
        assert_eq!(result.output().try_get_integer().unwrap(), 4);
    }

    #[tokio::test]
    async fn list_length() {
        let result = test_pattern(r#"list::count( $(self == 2) )"#, json!([1, 2])).await;
        assert_satisfied!(&result);
        assert_eq!(result.output().try_get_integer().unwrap(), 2);
    }

    #[tokio::test]
    async fn list_count_none_list() {
        let result = test_pattern(r#"list::count( $(self == 0) )"#, json!(123)).await;
        assert_satisfied!(&result);
        assert_eq!(result.output().try_get_integer().unwrap(), 0);
    }
}
