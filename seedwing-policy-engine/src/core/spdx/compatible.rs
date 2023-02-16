use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext, InnerType, ValueType};
use crate::runtime::{Output, RuntimeError, World};
use crate::value::RuntimeValue;
use spdx;
use spdx::{Expression, Licensee};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc; // as spdx_parser;

const DOCUMENTATION: &str = include_str!("compatible.adoc");

const LICENSE_REQUIREMENT: &str = "terms";

#[derive(Debug)]
pub struct Compatible;

impl Function for Compatible {
    fn order(&self) -> u8 {
        255
    }
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
            if let Some(value) = input.try_get_string() {
                if let Ok(license) = spdx::Expression::parse(value.as_str()) {
                    if let Some(val) = bindings.get(LICENSE_REQUIREMENT) {
                        if let Some(ValueType::String(license_req)) = val.try_get_resolved_value() {
                            if let Ok(license_id) = spdx::Licensee::parse(license_req.as_str()) {
                                if license.evaluate(|req| license_id.satisfies(req)) {
                                    return Ok(Output::Identity.into());
                                }
                            }
                        } else if let InnerType::List(license_list) = val.inner() {
                            let list: Vec<ValueType> = license_list
                                .to_vec()
                                .iter()
                                .filter_map(|t| t.try_get_resolved_value())
                                .collect();
                            let definitive_list: Vec<String> = list
                                .iter()
                                .filter_map(|t| {
                                    if let ValueType::String(val) = t {
                                        Some(val.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            for license_req in definitive_list {
                                if let Ok(license_id) = spdx::Licensee::parse(license_req.as_str())
                                {
                                    if license.evaluate(|req| license_id.satisfies(req)) {
                                        return Ok(Output::Identity.into());
                                    }
                                }
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

    use crate::core::test::test_pattern;

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
