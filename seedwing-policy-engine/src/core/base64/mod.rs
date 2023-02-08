use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::package::Package;
use crate::runtime::{Output, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::{RationaleResult, RuntimeValue};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::Arc;
use base64::engine::GeneralPurpose;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["base64"]));
    pkg.register_function("base64".into(), Base64::new(Alphabet::Standard));
    pkg.register_function("base64-url".into(), Base64::new(Alphabet::UrlNoPad));
    pkg.register_function("base64-encode".into(), Base64Encode);
    pkg
}

const DOCUMENTATION_BASE64: &str = include_str!("base64.adoc");
const DOCUMENTATION_BASE64_ENCODE: &str = include_str!("base64-encode.adoc");
const DOCUMENTATION_BASE64URL: &str = include_str!("base64-url.adoc");

#[derive(Debug)]
enum Alphabet {
    Standard,
    UrlNoPad,
}

impl Alphabet {
    pub fn decoder(&self) -> GeneralPurpose {
        match self {
            Alphabet::Standard => {
                STANDARD
            }
            Alphabet::UrlNoPad => {
                URL_SAFE_NO_PAD
            }
        }
    }
}

#[derive(Debug)]
pub struct Base64 {
    alphabet: Alphabet
}

impl Base64 {
    fn new(alphabet: Alphabet) -> Self {
        Self {
            alphabet
        }
    }
}

impl Function for Base64 {
    fn order(&self) -> u8 {
        128
    }

    fn documentation(&self) -> Option<String> {
        match self.alphabet {
            Alphabet::Standard=> Some(DOCUMENTATION_BASE64.into()),
            Alphabet::UrlNoPad => Some(DOCUMENTATION_BASE64URL.into()),
            _ => None
        }
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let input = (*input).borrow();
            if let Some(inner) = input.try_get_string() {
                let result = self.alphabet.decoder().decode(inner);

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
    fn order(&self) -> u8 {
        128
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION_BASE64_ENCODE.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let input = (*input).borrow();
            if let Some(inner) = input.try_get_octets() {
                let result = STANDARD.encode(inner);

                Ok(Output::Transform(Rc::new(result.into())).into())
            } else {
                Ok(Output::None.into())
            }
        })
    }
}
