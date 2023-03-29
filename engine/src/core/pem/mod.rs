use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::package::Package;
use crate::runtime::{EvalContext, Output, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::RuntimeValue;

use base64::engine::general_purpose::STANDARD_NO_PAD as PEM_ENGINE;
use base64::Engine;

use std::future::Future;
use std::pin::Pin;

use crate::lang::Severity;
use crate::runtime::rationale::Rationale;
use std::str::from_utf8;
use std::sync::Arc;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["pem"]));
    pkg.register_function("as-certificate".into(), AsCertificate);
    pkg
}

/// Encode a blob as PEM certificate
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct AsCertificate;

impl Function for AsCertificate {
    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let bytes = if let Some(inner) = input.try_get_octets() {
                inner
            } else {
                return Ok((
                    Severity::Error,
                    Rationale::InvalidArgument("Requires octets as input".to_string()),
                )
                    .into());
            };

            let contents = PEM_ENGINE.encode(bytes);
            // allocate a bit more than we actually need
            let mut result = String::with_capacity(contents.len() + 128);

            result.push_str("-----BEGIN CERTIFICATE-----\n");
            for c in contents.as_bytes().chunks(64) {
                // unwrapping from_utf8 should be safe a base64 encoding should only give back ASCII characters anyway
                result.push_str(from_utf8(c).unwrap());
                result.push('\n');
            }
            result.push_str("-----END CERTIFICATE-----\n");

            Ok(Output::Transform(Arc::new(result.into())).into())
        })
    }
}
