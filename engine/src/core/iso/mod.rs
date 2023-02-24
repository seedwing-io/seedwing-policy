use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["iso"]));
    pkg.register_source("swid".into(), include_str!("swid.dog"));
    pkg
}
