use crate::package::Package;
use crate::runtime::PackagePath;

mod find_advisory;
mod from_cve;
mod rhsa;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["rhsa"]));
    pkg.register_function("from-cve".into(), from_cve::FromCve);
    pkg.register_function("find-advisory".into(), find_advisory::FindAdvisory);
    pkg
}
