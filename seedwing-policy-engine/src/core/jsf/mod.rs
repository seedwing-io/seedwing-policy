use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["jsf"]));
    pkg.register_source("algorithm".into(), include_str!("algorithm.dog"));
    pkg.register_source("public-key".into(), include_str!("public_key.dog"));
    pkg.register_source("signaturecore".into(), include_str!("signaturecore.dog"));
    pkg
}
