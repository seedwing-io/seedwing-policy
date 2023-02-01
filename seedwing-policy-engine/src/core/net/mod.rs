use crate::package::Package;
use crate::runtime::PackagePath;

mod inet4addr;
mod url;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["net"]));
    pkg.register_function("Inet4Addr".into(), inet4addr::Inet4Addr);
    pkg.register_function("Url".into(), url::Url);
    pkg
}
