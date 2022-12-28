use crate::core::{Function, FunctionError};
use crate::lang::PackagePath;
use crate::package::Package;
use crate::runtime::{Bindings, EvaluationResult};
use crate::value::Value;
use std::future::Future;
use std::pin::Pin;

pub mod all;
pub mod any;
pub mod none;
pub mod some;

const COUNT: &str = "count";
const PATTERN: &str = "pattern";

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["list"]));
    pkg.register_function("Any".into(), any::Any);
    pkg.register_function("All".into(), all::All);
    pkg.register_function("None".into(), none::None);
    pkg.register_function("Some".into(), some::Some);
    pkg
}
