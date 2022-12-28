use crate::core::{Function, FunctionError};
use crate::lang::PackagePath;
use crate::package::Package;
use crate::runtime::Bindings;
use crate::value::Value;
use ariadne::Cache;
use std::future::Future;
use std::pin::Pin;
use std::str::from_utf8;
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
        input: &'v Value,
        bindings: &Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<Value, FunctionError>> + 'v>> {
        Box::pin(async move {
            let mut bytes = Vec::new();

            if let Some(inner) = input.try_get_string() {
                bytes.extend_from_slice(inner.as_bytes());
            } else if let Some(inner) = input.try_get_octets() {
                bytes.extend_from_slice(inner);
            } else {
                return Err(FunctionError::Other("invalid input type".into()));
            };

            let mut certs: Vec<Value> = Vec::new();

            for pem in Pem::iter_from_buffer(&bytes).flatten() {
                if pem.label == "PUBLIC" {
                    //println!("public key? {:?}", pem);
                } else if let Ok(x509) = pem.parse_x509() {
                    let converted: Value = (&x509).into();
                    certs.push(converted);
                }
            }

            Ok(certs.into())
        })
    }
}
