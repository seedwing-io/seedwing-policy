use crate::package::Package;
use crate::runtime::PackagePath;

mod rfc2822;
mod rfc3339;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["timestamp"]));
    pkg.register_function("Rfc3339".into(), rfc3339::Rfc3339);
    pkg.register_function("Iso8601".into(), rfc3339::Rfc3339);
    pkg.register_function("Rfc2822".into(), rfc2822::Rfc2822);
    pkg
}
