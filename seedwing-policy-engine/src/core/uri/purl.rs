use crate::core::uri::url::Url;
use crate::core::{BlockingFunction, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::runtime::rationale::Rationale;
use crate::runtime::{Output, RuntimeError, World};
use crate::value::{Object, RuntimeValue};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
pub struct Purl;

const DOCUMENTATION: &str = include_str!("Purl.adoc");

impl BlockingFunction for Purl {
    fn order(&self) -> u8 {
        0
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call(
        &self,
        input: Arc<RuntimeValue>,
        ctx: &mut EvalContext,
        bindings: &Bindings,
        world: &World,
    ) -> Result<FunctionEvaluationResult, RuntimeError> {
        match input.as_ref() {
            RuntimeValue::String(url) => match super::url::Url::parse_url(url) {
                Ok(url) => self.validate(&url),
                Err(result) => Ok(result),
            },
            RuntimeValue::Object(url) => self.validate(url),
            _ => Self::invalid_arg("input is neither a String nor an Object"),
        }
    }
}

impl Purl {
    fn validate(&self, url: &Object) -> Result<FunctionEvaluationResult, RuntimeError> {
        if url.has_str("host", "pkg") {
            return Self::invalid_arg(format!(
                "Purl invalid host value, must be 'pkg', has: {:?}",
                url.get("pkg")
            ));
        }

        let path = match url["path"].try_get_str() {
            Some(path) => path,
            None => return Self::invalid_arg(format!("Purl has no path")),
        };

        let mut result = Object::new();

        let name = match path.split('/').collect::<Vec<_>>().as_slice() {
            [r#type, name] => {
                result.set("type", *r#type);
                *name
            }
            [r#type, namespace, name] => {
                result.set("type", *r#type);
                result.set("namespace", *namespace);
                *name
            }
            _ => {
                return Self::invalid_arg(format!("Invalid Purl path"));
            }
        };

        // split name into name + version

        match name.splitn(2, '@').collect::<Vec<_>>().as_slice() {
            [name] => {
                result.set("name", *name);
                result.set("version", *name);
            }
            [name, version] => {
                result.set("name", *name);
                result.set("version", *version);
            }
            _ => {
                return Self::invalid_arg(format!("Invalid name syntax: {name}"));
            }
        }

        if let Some(subpath) = url["fragment"].try_get_str() {
            result.set("subpath", subpath);
        }

        match &url["query"] {
            RuntimeValue::String(query) => {
                result.set("qualifiers", Url::parse_query(query));
            }
            RuntimeValue::Object(query) => {
                result.set("qualifiers", query.clone());
            }
            _ => {}
        }

        Ok(Output::Transform(Arc::new(result.into())).into())
    }

    fn error(rationale: Rationale) -> Result<FunctionEvaluationResult, RuntimeError> {
        Ok((Output::None, rationale).into())
    }

    fn invalid_arg(msg: impl Into<String>) -> Result<FunctionEvaluationResult, RuntimeError> {
        Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::test::test_pattern;
    use serde_json::json;

    #[tokio::test]
    async fn test_purl_1() {
        let result = test_pattern(
            r#"uri::Purl"#,
            "pkg:rpm/fedora/curl@7.50.3-1.fc25?arch=i386&distro=fedora-25",
        )
        .await;

        assert_eq!(
            result.output(),
            Some(Rc::new(
                json!({
                    "type": "rpm",
                    "namespace": "fedora",
                    "name": "curl",
                    "version": "7.50.3-1.fc25",
                    "qualifiers": {
                        "arch": "i386",
                        "distro": "fedora-25",
                    },
                })
                .into()
            ))
        );
    }

    #[tokio::test]
    async fn test_purl_2() {
        let result = test_pattern(
            r#"uri::Purl"#,
            "pkg:docker/customer/dockerimage@sha256:244fd47e07d1004f0aed9c?repository_url=gcr.io",
        )
        .await;

        assert_eq!(
            result.output(),
            Some(Rc::new(
                json!({
                    "type": "docker",
                    "namespace": "customer",
                    "name": "dockerimage",
                    "version": "sha256:244fd47e07d1004f0aed9c",
                    "qualifiers": {
                        "repository_url": "gcr.io",
                    },
                })
                .into()
            ))
        );
    }

    #[tokio::test]
    async fn test_purl_3() {
        let result = test_pattern(r#"uri::Purl"#, "pkg:cargo/rand@0.7.2").await;

        assert_eq!(
            result.output(),
            Some(Rc::new(
                json!({
                    "type": "cargo",
                    "name": "rand",
                    "version": "0.7.2",
                })
                .into()
            ))
        );
    }

    #[tokio::test]
    async fn test_purl_4() {
        let result = test_pattern(
            r#"uri::Purl"#,
            "pkg:github/package-url/purl-spec@244fd47e07d1004#everybody/loves/dogs",
        )
        .await;

        assert_eq!(
            result.output(),
            Some(Rc::new(
                json!({
                    "type": "github",
                    "namespace": "package-url",
                    "name": "purl-spec",
                    "version": "244fd47e07d1004",
                    "subpath": "everybody/loves/dogs",
                })
                .into()
            ))
        );
    }
}
