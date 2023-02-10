use crate::core::{BlockingFunction, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::runtime::rationale::Rationale;
use crate::runtime::{Output, RuntimeError, World};
use crate::value::{Object, RuntimeValue};
use std::fmt::Debug;

use std::sync::Arc;

#[derive(Debug)]
pub struct Url;

const DOCUMENTATION: &str = include_str!("url.adoc");

impl BlockingFunction for Url {
    fn order(&self) -> u8 {
        0
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call(
        &self,
        input: Arc<RuntimeValue>,
        _ctx: &mut EvalContext,
        _bindings: &Bindings,
        _world: &World,
    ) -> Result<FunctionEvaluationResult, RuntimeError> {
        match input.as_ref() {
            RuntimeValue::String(value) => match Self::parse_url(value) {
                Ok(result) => Ok(Output::Transform(Arc::new(result.into())).into()),
                Err(result) => Ok(result),
            },
            _ => Ok((
                Output::None,
                Rationale::InvalidArgument(format!("input is not a String")),
            )
                .into()),
        }
    }
}

impl Url {
    pub fn parse_url(string: &str) -> Result<Object, FunctionEvaluationResult> {
        match ::url::Url::parse(string) {
            Ok(url) => {
                let mut result = Object::new();
                result.set("scheme", url.scheme());
                result.set("host", url.host_str());
                result.set("path", url.path());
                result.set("query", url.query());
                result.set("fragment", url.fragment());
                result.set("domain", url.domain());
                result.set("username", url.username());
                result.set("password", url.password());
                result.set("port", url.port());

                Ok(result)
            }
            Err(err) => Err((
                Output::None,
                Rationale::InvalidArgument(format!("input is not a URL: {err}")),
            )
                .into()),
        }
    }

    pub fn parse_query(query: &str) -> Object {
        let mut result = Object::new();

        for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
            result.set(k, v);
        }

        result
    }
}
