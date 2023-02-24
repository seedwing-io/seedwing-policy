use crate::package::Package;
use crate::runtime::PackagePath;

mod inet4addr;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["net"]));
    pkg.register_function("inet4addr".into(), inet4addr::Inet4Addr);
    pkg
}
