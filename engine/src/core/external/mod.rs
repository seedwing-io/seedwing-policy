mod eval;

use crate::core::external::eval::Eval;
use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["external"]));
    pkg.register_function("eval".into(), Eval);
    pkg
}
