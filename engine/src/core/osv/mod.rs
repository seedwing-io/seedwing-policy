use crate::package::Package;
use crate::runtime::PackagePath;

pub(crate) mod client;
mod purl;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["osv"])).with_documentation(
        "Patterns for working with the Open Source Vulnerability database (osv.dev).",
    );
    pkg.register_function("scan-purl".into(), purl::ScanPurl);
    pkg
}
