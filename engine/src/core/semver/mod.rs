mod parse;

use crate::core::semver::parse::SemverParse;
use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["semver"]));
    pkg.register_source("".into(), include_str!("semver.dog"));
    pkg.register_function("parse".into(), SemverParse);
    pkg
}
