use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::lang::{Severity, ValuePattern};
use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("concat.adoc");
const VALUE: &str = "value";

#[derive(Debug)]
pub enum Concat {
    Append,
    Prepend,
}

impl Function for Concat {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![VALUE.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(Some(ValuePattern::String(value))) =
                bindings.get(VALUE).map(|m| m.try_get_resolved_value())
            {
                if let Some(input) = input.try_get_string() {
                    let transformed = match self {
                        Self::Append => format!("{input}{value}"),
                        Self::Prepend => format!("{value}{input}"),
                    };
                    Ok(Output::Transform(Arc::new(transformed.into())).into())
                } else {
                    Ok(Severity::Error.into())
                }
            } else {
                Ok(Severity::Error.into())
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::assert_satisfied;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvalContext;
    use serde_json::json;

    #[tokio::test]
    async fn can_append() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern transformed = string::append<".json">
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate("test::transformed", json!("data"), EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(&result);
        let output = result.output();
        assert!(output.is_string());
        assert_eq!(output.as_json(), json!("data.json"));
    }

    #[tokio::test]
    async fn can_prepend() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern transformed = string::prepend<"my:">
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate("test::transformed", json!("data"), EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(&result);
        let output = result.output();
        assert!(output.is_string());
        assert_eq!(output.as_json(), json!("my:data"));
    }
}
