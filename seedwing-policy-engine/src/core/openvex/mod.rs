use crate::package::Package;
use crate::runtime::PackagePath;

mod openvex;
mod osv;
mod purl;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["openvex"]));
    pkg.register_source("openvex".into(), include_str!("openvex.dog"));
    pkg.register_function("FromPurl".into(), purl::FromPurl);
    pkg
}
