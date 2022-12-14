use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::lang::PackagePath;
use crate::package::Package;
use crate::runtime::{Output, RuntimeError};
use crate::value::{RationaleResult, RuntimeValue};
use ariadne::Cache;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::Arc;
use x509_parser::pem::Pem;
use x509_parser::x509::X509Version;

pub mod convert;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["x509"]));
    pkg.register_function("PEM".into(), PEM);
    pkg.register_source("oid".into(), include_str!("oid.dog"));
    pkg
}

#[derive(Debug)]
pub struct PEM;

impl Function for PEM {
    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let mut bytes = Vec::new();

            if let Some(inner) = input.try_get_string() {
                bytes.extend_from_slice(inner.as_bytes());
            } else if let Some(inner) = input.try_get_octets() {
                bytes.extend_from_slice(inner);
            } else {
                return Ok(Output::None.into());
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
                Ok(Output::None.into())
            } else {
                Ok(Output::Transform(Rc::new(certs.into())).into())
            }
        })
    }
}
