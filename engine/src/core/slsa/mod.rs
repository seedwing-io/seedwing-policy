use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["slsa"]));
    pkg.register_source("provenance".into(), include_str!("provenance-v1_0.dog"));
    pkg
}
