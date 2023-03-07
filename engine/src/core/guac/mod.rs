use crate::package::Package;
use crate::runtime::PackagePath;

mod certify_vuln;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["guac"]));
    pkg.register_function("certify-vulnerability".into(), certify_vuln::CertifyVuln);
    pkg
}
