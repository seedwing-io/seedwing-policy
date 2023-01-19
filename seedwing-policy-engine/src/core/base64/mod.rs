use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::package::Package;
use crate::runtime::{Output, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::{RationaleResult, RuntimeValue};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::Arc;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["base64"]));
    pkg.register_function("Base64".into(), Base64);
    pkg.register_function("Base64Encode".into(), Base64Encode);
    pkg
}

const DOCUMENTATION_BASE64: &str = include_str!("Base64.adoc");
const DOCUMENTATION_BASE64_ENCODE: &str = include_str!("Base64Encode.adoc");

#[derive(Debug)]
pub struct Base64;

impl Function for Base64 {
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION_BASE64.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let input = (*input).borrow();
            if let Some(inner) = input.try_get_string() {
                let result = base64::decode(inner);

                if let Ok(decoded) = result {
                    Ok(Output::Transform(Rc::new(decoded.into())).into())
                } else {
                    //Err(FunctionError::Other("unable to decode base64".into()))
                    Ok(Output::None.into())
                }
            } else {
                Ok(Output::None.into())
            }
        })
    }
}

#[derive(Debug)]
pub struct Base64Encode;

impl Function for Base64Encode {
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION_BASE64_ENCODE.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let input = (*input).borrow();
            if let Some(inner) = input.try_get_octets() {
                let result = base64::encode(inner);

                Ok(Output::Transform(Rc::new(result.into())).into())
            } else {
                Ok(Output::None.into())
            }
        })
    }
}
