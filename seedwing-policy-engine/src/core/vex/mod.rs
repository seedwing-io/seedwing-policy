use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["vex"]));
    pkg.register_source("v0_0_0".into(), include_str!("v0_0_0.dog"));
    pkg
}
