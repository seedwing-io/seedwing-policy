use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::str::from_utf8;
use crate::function::{Function, FunctionPackage};
use crate::value::Value;

pub fn package() -> FunctionPackage {
    let mut pkg = FunctionPackage::new();
    pkg.register("Base64".into(), Base64);
    pkg
}

#[derive(Debug)]
pub struct Base64;

impl Function for Base64 {
    fn call<'v>(&'v self, value: &'v mut Value) -> Pin<Box<dyn Future<Output=Result<Value, ()>> + 'v>> {
        Box::pin(
            async move {
                if let Some(inner) = value.try_get_string() {
                    if let Ok(decoded) = base64::decode(inner) {
                        Ok(decoded.into())
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            }
        )
    }
}