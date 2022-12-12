use std::collections::HashMap;
use std::future::Future;
use crate::value::Value;

pub mod sigstore;

pub trait Function {
    fn call<'v>(&'v self, input: &'v mut Value) -> Box<dyn Future<Output=Result<Value, ()>> + 'v>;
}

pub struct FunctionPackage {
    fns: HashMap<String, Box<dyn Function>>,
}

impl FunctionPackage {

    pub fn new() -> Self {
        Self {
            fns: Default::default()
        }
    }

    pub fn register<F: Function + 'static>(&mut self, name: String, func: F) {
        self.fns.insert( name, Box::new(func));
    }

    pub fn function_names(&self) -> Vec<String> {
        self.fns.keys().cloned().collect()
    }

}