use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern, ValuePattern};
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;
use spdx;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc; // as spdx_parser;

const DOCUMENTATION: &str = include_str!("compatible.adoc");

const LICENSE_REQUIREMENT: &str = "license_id";

#[derive(Debug)]
pub struct Compatible;

impl Function for Compatible {
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn parameters(&self) -> Vec<String> {
        vec![LICENSE_REQUIREMENT.into()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            // Gather parameters
            let authorized_licenses = if let Some(val) = bindings.get(LICENSE_REQUIREMENT) {
                match val.inner() {
                    InnerPattern::List(license_list) => license_list
                        .to_vec()
                        .iter()
                        .filter_map(|t| t.try_get_resolved_value())
                        .filter_map(|t| match t {
                            ValuePattern::String(val) => Some(val),
                            _ => None,
                        })
                        .collect::<Vec<String>>(),
                    InnerPattern::Const(ValuePattern::String(license)) => vec![license.clone()],
                    _ => return Ok(Output::None.into()),
                }
            } else {
                return Ok(Output::None.into());
            };

            // the input should be a string
            if let Some(value) = input.try_get_string() {
                if let Ok(license) = spdx::Expression::parse(value.as_str()) {
                    for license_req in authorized_licenses {
                        if let Ok(license_id) = spdx::Licensee::parse(license_req.as_str()) {
                            if license.evaluate(|req| license_id.satisfies(req)) {
                                return Ok(Output::Identity.into());
                            }
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

    use crate::runtime::testutil::test_pattern;

    use serde_json::json;

    #[actix_rt::test]
    async fn gpl() {
        let result = test_pattern(
            r#"
            spdx::compatible<"GPL-2.0">
            "#,
            json!("GPL-2.0-only"),
        )
        .await;

        assert!(result.satisfied())
    }

    #[actix_rt::test]
    async fn fail() {
        let result = test_pattern(
            r#"
            spdx::compatible<"MIT">
            "#,
            json!("Apache-2.0"),
        )
        .await;

        assert!(!result.satisfied())
    }

    #[actix_rt::test]
    async fn multiple() {
        let result = test_pattern(
            r#"
            spdx::compatible<["MIT", "GPL-2.0"]>
            "#,
            json!("MIT OR Apache-2.0"),
        )
        .await;

        assert!(result.satisfied())
    }

    #[actix_rt::test]
    async fn multiple_fails() {
        let result = test_pattern(
            r#"
            spdx::compatible<["BSD", "GPL-2.0"]>
            "#,
            json!("MIT OR Apache-2.0"),
        )
        .await;

        assert!(!result.satisfied())
    }
}
