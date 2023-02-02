use crate::package::Package;
use crate::runtime::PackagePath;

mod iri;
mod purl;
mod url;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["uri"]));
    pkg.register_function("Url".into(), url::Url);
    pkg.register_function("Iri".into(), iri::Iri);
    pkg.register_function("Purl".into(), purl::Purl);
    pkg
}
