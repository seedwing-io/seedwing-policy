use crate::runtime::RuntimeType::Primordial;
use crate::runtime::{Bindings, RuntimeType};
use crate::value::Value;
use async_mutex::Mutex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

pub mod base64;
pub mod sigstore;
pub mod x509;

#[derive(Debug)]
pub enum FunctionError {
    Other(String),
}

pub trait Function: Sync + Send + Debug {
    fn parameters(&self) -> Vec<String> {
        Default::default()
    }

    fn call<'v>(
        &'v self,
        input: &'v Value,
        bindings: &Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<Value, FunctionError>> + 'v>>;
}
