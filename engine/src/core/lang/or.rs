use crate::core::{Function, FunctionEvaluationResult, FunctionInput, FunctionInputPattern};
use crate::lang::lir::{Bindings, InnerPattern};
use crate::runtime::{EvalContext, Output, Pattern, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;

use crate::lang::PatternMeta;
use crate::lang::PrimordialPattern;
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
            documentation: Some(DOCUMENTATION.into()),
            ..Default::default()
        }
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(terms) = bindings.get(TERMS) {
                if let InnerPattern::List(terms) = terms.inner() {
                    let mut rationale = Vec::new();
                    let mut terms = terms.clone();
                    terms.sort_by_key(|a| a.order(world));
                    for term in terms {
                        let result = term.evaluate(input.clone(), ctx, bindings, world).await?;
                        if result.satisfied() {
                            rationale.push(result);
                            return Ok((Output::Identity, rationale).into());
                        }
                        rationale.push(result);
                    }

                    return Ok((Output::None, rationale).into());
                }
            }

            Ok(Output::None.into())
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use serde_json::json;

    #[actix_rt::test]
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
            .await;

        assert!(result.unwrap().satisfied())
    }
}
