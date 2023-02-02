use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["vex"]));
    pkg.register_source("openvex".into(), include_str!("openvex.dog"));
    pkg
}
