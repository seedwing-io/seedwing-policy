use crate::data::DataSource;
use crate::package::Package;
use crate::runtime::PackagePath;

mod from;

use crate::core::data::from::From;

pub fn package(data_sources: Vec<Box<dyn DataSource>>) -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["data"]));
    pkg.register_function("From".into(), From::new(data_sources));
    //pkg.register_source("v1_4".into(), include_str!("v1_4.dog"));
    //pkg.register_source("hash".into(), include_str!("hash.dog"));
    pkg
}
