use crate::core::{Function, FunctionError};
use crate::value::Value;
use async_mutex::Mutex;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::Arc;
use crate::lang::ty::PackagePath;
use crate::package::Package;

pub fn package() -> Package {
    let mut pkg = Package::new(
        PackagePath::from_parts( vec![
            "base64",
        ])
    );
    pkg.register_function("Base64".into(), Base64);
    pkg
}

#[derive(Debug)]
pub struct Base64;

impl Function for Base64 {
    fn call<'v>(
        &'v self,
        input: &'v Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value, FunctionError>> + 'v>> {
        Box::pin(async move {
            if let Some(inner) = input.try_get_string() {
                if let Ok(decoded) = base64::decode(inner) {
                    Ok(decoded.into())
                } else {
                    Err(FunctionError::Other("unable to decode base64".into()))
                }
            } else {
                Err(FunctionError::Other("invalid input type".into()))
            }
        })
    }
}
