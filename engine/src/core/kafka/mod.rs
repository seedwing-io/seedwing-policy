use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["kafka"]));
    pkg.register_source("opa".into(), include_str!("opa.dog"));
    pkg
}
