use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use crate::runtime::RuntimeType::Primordial;
use crate::value::Value;

pub mod sigstore;

pub trait Function : Debug {
    fn call<'v>(&'v self, input: &'v mut Value) -> Pin<Box<dyn Future<Output=Result<Value, ()>> + 'v >>;
}

pub struct FunctionPackage {
    fns: HashMap<String, Arc<dyn Function>>,
}

impl FunctionPackage {
    pub fn new() -> Self {
        Self {
            fns: Default::default()
        }
    }

    pub fn register<F: Function + 'static>(&mut self, name: String, func: F) {
        self.fns.insert(name, Arc::new(func));
    }

    pub fn function_names(&self) -> Vec<String> {
        self.fns.keys().cloned().collect()
    }

    pub fn functions(&self) -> Vec<(String, Arc<dyn Function>)> {
        self.fns
            .iter()
            .map(|(k, v)| {
                (k.clone(), v.clone())
            })
            .collect()
    }
}