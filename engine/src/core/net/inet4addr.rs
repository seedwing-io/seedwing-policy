use crate::runtime::EvalContext;
use crate::{
    core::{Function, FunctionEvaluationResult},
    lang::lir::{Bindings, ValuePattern},
    runtime::{rationale::Rationale, EvaluationResult, Output, RuntimeError, World},
    value::RuntimeValue,
};
use cidr::*;
use std::{future::Future, pin::Pin, sync::Arc};
use std::{net::Ipv4Addr, str::FromStr};

const DOCUMENTATION: &str = include_str!("inet4addr.adoc");
const ADDRESS: &str = "address";

#[derive(Debug)]
pub struct Inet4Addr;

impl Function for Inet4Addr {
    fn order(&self) -> u8 {
        128
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn parameters(&self) -> Vec<String> {
        vec![ADDRESS.to_string()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(address_pattern) = bindings.get(ADDRESS) {
                if let Some(ValuePattern::String(range)) = address_pattern.try_get_resolved_value()
                {
                    match Ipv4Cidr::from_str(&range) {
                        Ok(range) => {
                            if let Some(addr) = input.try_get_string() {
                                return match Ipv4Addr::from_str(&addr) {
                                    Ok(addr) => {
                                        if range.contains(&addr) {
                                            Ok(Output::Identity.into())
                                        } else {
                                            Ok(Output::None.into())
                                        }
                                    }
                                    Err(e) => {
                                        return Ok((
                                            Output::None,
                                            vec![EvaluationResult::new(
                                                input,
                                                address_pattern,
                                                Rationale::InvalidArgument(e.to_string()),
                                                Output::None,
                                            )],
                                        )
                                            .into())
                                    }
                                };
                            }
                        }
                        Err(e) => {
                            let e = format!("error parsing inet4addr<\"{range}\">: {e}");
                            return Ok((
                                Output::None,
                                vec![EvaluationResult::new(
                                    input,
                                    address_pattern,
                                    Rationale::InvalidArgument(e),
                                    Output::None,
                                )],
                            )
                                .into());
                        }
                    }
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
    async fn test_exact_match() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern allow = net::inet4addr<"10.0.0.1">
        "#,
        );

        let result = eval(src.clone(), "test::allow", "10.0.0.1").await.unwrap();
        assert!(result.satisfied(), "{:?}", result.rationale());

        let result = eval(src, "test::allow", "10.0.0.2").await.unwrap();
        assert!(!result.satisfied(), "{:?}", result.rationale());
    }

    #[actix_rt::test]
    async fn test_cidr_match() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern allow = net::inet4addr<"10.0.0.0/16">
        "#,
        );

        assert!(eval(src.clone(), "test::allow", "10.0.0.1")
            .await
            .unwrap()
            .satisfied());

        assert!(!eval(src, "test::allow", "10.1.0.1")
            .await
            .unwrap()
            .satisfied());
    }

    #[actix_rt::test]
    async fn test_invalid_matcher() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern allow = net::inet4addr<"10.0.0.1/16">
        "#,
        );

        assert!(!eval(src, "test::allow", "10.0.0.1")
            .await
            .unwrap()
            .satisfied());
    }

    #[actix_rt::test]
    async fn test_invalid_input() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern allow = net::inet4addr<"10.0.0.0/16">
        "#,
        );

        assert!(!eval(src, "test::allow", "10.0.0.a")
            .await
            .unwrap()
            .satisfied());
    }

    async fn eval(
        src: Ephemeral,
        path: &str,
        value: &str,
    ) -> Result<EvaluationResult, RuntimeError> {
        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        runtime
            .evaluate(path, json!(value), EvalContext::default())
            .await
    }
}
