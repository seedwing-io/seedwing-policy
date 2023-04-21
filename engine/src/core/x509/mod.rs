use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::{lir::Bindings, Severity};
use crate::package::Package;
use crate::runtime::{ExecutionContext, Output, PackagePath, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use x509_parser::parse_x509_certificate;
use x509_parser::pem::Pem;

pub mod convert;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["x509"]));
    pkg.register_function("pem".into(), PEM);
    pkg.register_function("der".into(), DER);
    pkg.register_source("oid".into(), include_str!("oid.dog"));
    pkg.register_source("".into(), include_str!("certificate.dog"));
    pkg
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct PEM;

impl Function for PEM {
    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: ExecutionContext<'v>,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let mut bytes = Vec::new();

            if let Some(inner) = input.try_get_str() {
                bytes.extend_from_slice(inner.as_bytes());
            } else if let Some(inner) = input.try_get_octets() {
                bytes.extend_from_slice(inner);
            } else {
                return Ok(Severity::Error.into());
            };

            let mut certs: Vec<RuntimeValue> = Vec::new();

            for pem in Pem::iter_from_buffer(&bytes).flatten() {
                if pem.label == "PUBLIC" {
                    //println!("public key? {:?}", pem);
                } else if let Ok(x509) = pem.parse_x509() {
                    let converted: RuntimeValue = (&x509).into();
                    certs.push(converted);
                }
            }

            if certs.is_empty() {
                Ok(Severity::Error.into())
            } else {
                Ok(Output::Transform(Arc::new(certs.into())).into())
            }
        })
    }
}

/// Decode a single DER encoded X.509 certificate
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub struct DER;

impl Function for DER {
    fn order(&self) -> u8 {
        128
    }
    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: ExecutionContext<'v>,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let bytes = if let Some(inner) = input.try_get_octets() {
                inner
            } else {
                return Ok(Severity::Error.into());
            };

            match parse_x509_certificate(bytes) {
                Ok((_, cert)) => Ok(Output::Transform(Arc::new((&cert).into())).into()),
                Err(_) => Ok(Severity::Error.into()),
            }
        })
    }
}
