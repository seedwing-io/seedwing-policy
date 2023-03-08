use crate::data::DataSource;
use crate::package::Package;
use crate::runtime::PackagePath;
use std::sync::Arc;

mod from;
mod lookup;

use crate::core::data::from::From;
use crate::core::data::lookup::Lookup;

pub fn package(data_sources: Vec<Arc<dyn DataSource>>) -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["data"]));
    pkg.register_function("from".into(), From::new(data_sources.clone()));
    pkg.register_function("lookup".into(), Lookup::new(data_sources.clone()));
    pkg
}
