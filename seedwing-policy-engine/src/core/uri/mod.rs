use crate::package::Package;
use crate::runtime::PackagePath;

mod iri;
mod purl;
mod url;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["uri"]));
    pkg.register_function("url".into(), url::Url);
    pkg.register_function("iri".into(), iri::Iri);
    pkg.register_function("purl".into(), purl::Purl);
    pkg
}
