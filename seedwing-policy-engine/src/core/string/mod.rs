mod length;

use crate::core::string::length::Length;
use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::package::Package;
use crate::runtime::{Output, PackagePath, RuntimeError, World};
use crate::value::RuntimeValue;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["string"]));
    pkg.register_function("Length".into(), Length);
    pkg
}
