use crate::package::Package;
use crate::runtime::PackagePath;

pub(crate) mod client;
mod purl;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["osv"]));
    pkg.register_function("from-purl".into(), purl::FromPurl);
    pkg
}
