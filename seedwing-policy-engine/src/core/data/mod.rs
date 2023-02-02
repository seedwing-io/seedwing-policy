use crate::data::DataSource;
use crate::package::Package;
use crate::runtime::PackagePath;
use std::sync::Arc;

mod from;

use crate::core::data::from::From;

pub fn package(data_sources: Vec<Arc<dyn DataSource>>) -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["data"]));
    pkg.register_function("From".into(), From::new(data_sources));
    pkg
}
