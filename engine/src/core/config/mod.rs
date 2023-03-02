#[allow(clippy::module_inception)]
mod of;

use crate::core::config::of::Of;
use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["config"]));
    pkg.register_function("of".into(), Of);
    pkg
}
