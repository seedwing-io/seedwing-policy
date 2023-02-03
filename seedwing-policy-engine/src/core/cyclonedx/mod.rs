use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["cyclonedx"]));
    pkg.register_source("v1_4".into(), include_str!("v1_4.dog"));
    pkg.register_source("v1_4/structure".into(), include_str!("v1_4/structure.dog"));
    pkg.register_source("hash".into(), include_str!("hash.dog"));
    pkg
}
