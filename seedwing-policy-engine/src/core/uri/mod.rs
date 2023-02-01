use crate::package::Package;
use crate::runtime::PackagePath;

mod url;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["uri"]));
    pkg.register_function("Url".into(), url::Url);
    pkg
}
