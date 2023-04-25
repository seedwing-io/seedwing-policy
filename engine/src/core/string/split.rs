use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern, ValuePattern};
use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("split.adoc");
const PATTERN: &str = "pattern";

#[derive(Debug)]
pub struct Split;

impl Function for Split {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(pattern) = bindings.get(PATTERN) {
                if let InnerPattern::Const(ValuePattern::String(pattern)) = pattern.inner() {
                    if let Some(string) = input.try_get_str() {
                        let list = string
                            .split(pattern.as_ref())
                            .map(|s| Arc::new(RuntimeValue::String(s.into())))
                            .collect();
                        return Ok(Output::Transform(Arc::new(RuntimeValue::List(list))).into());
                    }
                }
            }
            Ok(Output::Transform(Arc::new(RuntimeValue::List(Vec::new()))).into())
        })
    }
}

#[cfg(test)]
mod test {
    use crate::assert_satisfied;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvalContext;
    use crate::value::RuntimeValue;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn string_split() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern sp = string::split<",">
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::sp", json!("1,2,3,4"), EvalContext::default())
            .await
            .unwrap();
        assert_satisfied!(&result);

        let output = result.output();
        let list = output.try_get_list().unwrap();
        assert_eq!(list.len(), 4);
        assert!(list.contains(&Arc::new(RuntimeValue::String("1".into()))));
        assert!(list.contains(&Arc::new(RuntimeValue::String("2".into()))));
        assert!(list.contains(&Arc::new(RuntimeValue::String("3".into()))));
        assert!(list.contains(&Arc::new(RuntimeValue::String("4".into()))));
    }

    #[tokio::test]
    async fn string_split_no_pattern() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern sp = string::split()
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::sp", json!("1,2,3,4"), EvalContext::default())
            .await
            .unwrap();
        assert_satisfied!(&result);

        let output = result.output();
        let list = output.try_get_list().unwrap();
        assert_eq!(list.len(), 0);
    }

    #[tokio::test]
    async fn string_split_no_pattern_found() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern sp = string::split<",">
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::sp", json!("1:2:3:4"), EvalContext::default())
            .await
            .unwrap();
        assert_satisfied!(&result);

        let output = result.output();
        let list = output.try_get_list().unwrap();
        assert_eq!(list.len(), 1);
        assert!(list.contains(&Arc::new(RuntimeValue::String("1:2:3:4".into()))));
    }
}
