mod gav;

use crate::core::maven::gav::GAV;
use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["maven"]));
    pkg.register_source("".into(), include_str!("maven.dog"));
    pkg.register_function("GAV".into(), GAV);
    pkg
}
