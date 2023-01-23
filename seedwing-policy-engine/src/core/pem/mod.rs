use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::package::Package;
use crate::runtime::{Output, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::{RationaleResult, RuntimeValue};
use ariadne::Cache;
use base64::alphabet::STANDARD;
use base64::engine::fast_portable::{FastPortable, NO_PAD};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::Arc;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["pem"]));
    pkg.register_function("AsCertificate".into(), AsCertificate);
    pkg
}

/// Encode a blob as PEM certificate
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct AsCertificate;

const PEM_ENGINE: FastPortable = FastPortable::from(&STANDARD, NO_PAD);

impl Function for AsCertificate {
    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let bytes = if let Some(inner) = input.try_get_octets() {
                inner
            } else {
                return Ok(Output::None.into());
            };

            let contents = base64::encode_engine(&bytes, &PEM_ENGINE);
            // allocate a bit more than we actually need
            let mut result = String::with_capacity(contents.len() + 128);

            result.push_str("-----BEGIN CERTIFICATE-----\n");
            for c in contents.as_bytes().chunks(64) {
                // unwrapping from_utf8 should be safe a base64 encoding should only give back ASCII characters anyway
                result.push_str(from_utf8(c).unwrap());
                result.push('\n');
            }
            result.push_str("-----END CERTIFICATE-----\n");

            Ok(Output::Transform(Rc::new(result.into())).into())
        })
    }
}
