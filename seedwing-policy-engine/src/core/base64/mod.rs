use crate::core::{Function, FunctionError};
use crate::lang::lir::Bindings;
use crate::lang::PackagePath;
use crate::package::Package;
use crate::value::{RationaleResult, Value};
use async_mutex::Mutex;
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
    pkg
}

const DOCUMENTATION: &str = include_str!("Base64.adoc");

#[derive(Debug)]
pub struct Base64;

impl Function for Base64 {
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Arc<Mutex<Value>>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<RationaleResult, FunctionError>> + 'v>> {
        Box::pin(async move {
            let input = input.lock().await;
            if let Some(inner) = input.try_get_string() {
                let result = base64::decode(inner);

                if let Ok(decoded) = result {
                    Ok(RationaleResult::Transform(Arc::new(Mutex::new(
                        decoded.into(),
                    ))))
                } else {
                    Err(FunctionError::Other("unable to decode base64".into()))
                }
            } else {
                Err(FunctionError::InvalidInput)
            }
        })
    }
}
