use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["jsf"]));
    pkg.register_source("".into(), include_str!("algorithm.dog"));
    pkg.register_source("".into(), include_str!("public_key.dog"));
    pkg
}
