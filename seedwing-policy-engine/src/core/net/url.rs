use crate::core::{Function, FunctionEvaluationResult, SyncFunction};
use crate::lang::lir::{Bindings, EvalContext};
use crate::runtime::rationale::Rationale;
use crate::runtime::{EvaluationResult, Output, RuntimeError, World};
use crate::value::{Object, RuntimeValue};
use std::fmt::{format, Debug};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

#[derive(Debug)]
pub struct Url;

const DOCUMENTATION: &str = include_str!("Url.adoc");

impl SyncFunction for Url {
    fn order(&self) -> u8 {
        0
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call(
        &self,
        input: Rc<RuntimeValue>,
        ctx: &mut EvalContext,
        bindings: &Bindings,
        world: &World,
    ) -> Result<FunctionEvaluationResult, RuntimeError> {
        match input.as_ref() {
            RuntimeValue::String(value) => match ::url::Url::parse(&value) {
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

                    Ok(Output::Transform(Rc::new(result.into())).into())
                }
                Err(err) => Ok((
                    Output::None,
                    Rationale::InvalidArgument(format!("input is not a URL: {err}")),
                )
                    .into()),
            },
            _ => Ok((
                Output::None,
                Rationale::InvalidArgument(format!("input is not a String")),
            )
                .into()),
        }
    }
}
