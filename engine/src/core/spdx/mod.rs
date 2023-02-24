use crate::package::Package;
use crate::runtime::PackagePath;

mod compatible;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["spdx"]));
    pkg.register_source("license".into(), include_str!("license.dog"));
    pkg.register_function("compatible".into(), compatible::Compatible);
    pkg
}
