use crate::core::{Function, FunctionEvaluationResult, FunctionInput};
use crate::lang::lir::Bindings;
use crate::runtime::{ExecutionContext, Output, Pattern, RuntimeError, World};
use crate::value::{Object, RuntimeValue};
use std::future::Future;
use std::pin::Pin;

use semver::Version;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

#[derive(Debug)]
pub struct SemverParse;
const DOCUMENTATION: &str = include_str!("parse.adoc");

impl Function for SemverParse {
    fn input(&self, _bindings: &[Arc<Pattern>]) -> FunctionInput {
        FunctionInput::String
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
            if let Some(value) = input.try_get_string() {
                if let Ok(version) = Version::parse(value.as_str()) {
                    let mut semantic_version = Object::new();
                    semantic_version.set::<&str, u64>("major", version.major);
                    semantic_version.set::<&str, u64>("minor", version.minor);
                    semantic_version.set::<&str, u64>("patch", version.patch);
                    if !version.pre.is_empty() {
                        semantic_version.set::<&str, &str>("pre", version.pre.as_str());
                    }
                    if !version.build.is_empty() {
                        semantic_version.set::<&str, &str>("build", version.build.as_str());
                    }

                    return Ok(Output::Transform(Arc::new(semantic_version.into())).into());
                }
            }
            Ok(Severity::Error.into())
        })
    }
}

#[cfg(test)]
mod test {
    use crate::assert_satisfied;
    use crate::runtime::testutil::test_common;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    pub async fn test_semver() {
        let result = test_common(
            r#"
pattern test = semver::parse
"#,
            "0.1.2",
        )
        .await;

        assert_satisfied!(&result);
        assert_eq!(
            result.output(),
            Arc::new(
                json!({
                    "major": 0,
                    "minor": 1,
                    "patch": 2,
                })
                .into()
            )
        )
    }

    #[tokio::test]
    pub async fn test_semver_with_beta() {
        let result = test_common(
            r#"
pattern test = semver::parse
"#,
            "0.1.2-beta1",
        )
        .await;

        assert_satisfied!(&result);
        assert_eq!(
            result.output(),
            Arc::new(
                json!({
                    "major": 0,
                    "minor": 1,
                    "patch": 2,
                    "pre": "beta1"
                })
                .into()
            )
        )
    }

    #[tokio::test]
    pub async fn test_semver_with_build() {
        let result = test_common(
            r#"
pattern test = semver::parse
"#,
            "0.1.2+01042023",
        )
        .await;

        assert_satisfied!(&result);
        assert_eq!(
            result.output(),
            Arc::new(
                json!({
                    "major": 0,
                    "minor": 1,
                    "patch": 2,
                    "build": "01042023"
                })
                .into()
            )
        )
    }

    #[tokio::test]
    pub async fn test_semver_with_pre_and_build() {
        let result = test_common(
            r#"
pattern test = semver::parse
"#,
            "0.1.2-beta+01042023",
        )
        .await;

        assert_satisfied!(&result);
        assert_eq!(
            result.output(),
            Arc::new(
                json!({
                    "major": 0,
                    "minor": 1,
                    "patch": 2,
                    "pre": "beta",
                    "build": "01042023"
                })
                .into()
            )
        )
    }
}
