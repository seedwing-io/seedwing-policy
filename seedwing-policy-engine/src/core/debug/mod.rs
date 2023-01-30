use crate::package::Package;
use crate::runtime::PackagePath;

use delay::*;

mod delay;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["debug"]));
    pkg.register_function("DelayMs".into(), DelayMs);
    pkg
}
