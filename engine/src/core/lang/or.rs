use crate::core::{Function, FunctionEvaluationResult, FunctionInput, FunctionInputPattern};
use crate::lang::lir::{Bindings, InnerPattern};
use crate::runtime::{ExecutionContext, Pattern, RuntimeError, World};
use crate::value::RuntimeValue;
use std::cmp::max;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PrimordialPattern;
use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("or.adoc");

const TERMS: &str = "terms";

#[derive(Debug)]
pub struct Or;

impl Function for Or {
    fn input(&self, bindings: &[Arc<Pattern>]) -> FunctionInput {
        FunctionInput::Pattern(FunctionInputPattern::Or(
            bindings
                .iter()
                .flat_map(|e| match e.inner() {
                    InnerPattern::Primordial(inner) => match inner {
                        PrimordialPattern::Integer => Some(FunctionInput::Integer),
                        PrimordialPattern::Decimal => Some(FunctionInput::Decimal),
                        PrimordialPattern::Boolean => Some(FunctionInput::Boolean),
                        PrimordialPattern::String => Some(FunctionInput::String),
                        PrimordialPattern::Function(_, _, func) => Some(func.input(bindings)),
                    },
                    _ => None,
                })
                .collect(),
        ))
    }

    fn parameters(&self) -> Vec<String> {
        vec![TERMS.into()]
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
        ctx: ExecutionContext<'v>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(terms) = bindings.get(TERMS) {
                if let InnerPattern::List(terms) = terms.inner() {
                    let mut supporting = Vec::new();
                    let mut terms = terms.clone();
                    terms.sort_by_key(|a| a.order(world));

                    let mut satisfied = false;
                    let mut base_severity = Severity::None;

                    for term in terms {
                        // eval term
                        let result = term
                            .evaluate(input.clone(), ctx.push()?, bindings, world)
                            .await?;

                        let severity = result.severity();
                        if !matches!(severity, Severity::Error) {
                            // not failed, so we have at least one
                            satisfied = true;
                            // record highest (not failed) severity
                            base_severity = max(base_severity, severity);
                        }

                        // record result
                        supporting.push(result);
                    }

                    // eval our severity (max non-failed, or failed)
                    let severity = match satisfied {
                        true => base_severity,
                        false => Severity::Error,
                    };

                    return Ok((severity, supporting).into());
                }
            }

            Ok(Severity::Error.into())
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
    async fn list_operation() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern test-or = lang::or<["foo", "bar"]>
        "#,
        );

        let mut builder = Builder::new();

        let _result = builder.build(src.iter());

        let runtime = builder.finish().await.unwrap();

        let value = json!("foo");

        let result = runtime
            .evaluate("test::test-or", value, EvalContext::default())
            .await
            .unwrap();

        assert_satisfied!(result);
    }
}
