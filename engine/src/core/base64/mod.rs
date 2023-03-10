use crate::core::{Example, Function, FunctionEvaluationResult, FunctionInput};
use crate::lang::lir::Bindings;
use crate::package::Package;
use crate::runtime::{EvalContext, Output, Pattern, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::RuntimeValue;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::engine::GeneralPurpose;
use base64::Engine;
use std::borrow::Borrow;

use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

use serde_json::json;
use std::sync::Arc;

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
            Alphabet::Standard => STANDARD,
            Alphabet::UrlNoPad => URL_SAFE_NO_PAD,
        }
    }
}

#[derive(Debug)]
pub struct Base64 {
    alphabet: Alphabet,
}

impl Base64 {
    fn new(alphabet: Alphabet) -> Self {
        Self { alphabet }
    }
}

impl Function for Base64 {
    fn input(&self, _bindings: &Vec<Arc<Pattern>>) -> FunctionInput {
        FunctionInput::String
    }

    fn documentation(&self) -> Option<String> {
        match self.alphabet {
            Alphabet::Standard => Some(DOCUMENTATION_BASE64.into()),
            Alphabet::UrlNoPad => Some(DOCUMENTATION_BASE64URL.into()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            name: "default".to_string(),
            summary: Some("Simple base64 encoded value".to_string()),
            description: None,
            value: json!("SGVsbG8gUm9kbmV5IQ=="),
        }]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let input = (*input).borrow();
            if let Some(inner) = input.try_get_string() {
                let result = self.alphabet.decoder().decode(inner);

                if let Ok(decoded) = result {
                    Ok(Output::Transform(Arc::new(decoded.into())).into())
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
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let input = (*input).borrow();
            if let Some(inner) = input.try_get_octets() {
                let result = STANDARD.encode(inner);

                Ok(Output::Transform(Arc::new(result.into())).into())
            } else {
                Ok(Output::None.into())
            }
        })
    }
}
