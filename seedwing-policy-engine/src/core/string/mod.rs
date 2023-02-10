mod length;
mod regexp;

use crate::core::string::length::Length;
use crate::core::string::regexp::Regexp;

use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["string"]));
    pkg.register_function("length".into(), Length);
    pkg.register_function("regexp".into(), Regexp);
    pkg
}
