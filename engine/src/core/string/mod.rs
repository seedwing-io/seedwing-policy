mod concat;
mod contains;
mod length;
mod regexp;
mod split;

use crate::core::string::concat::Concat;
use crate::core::string::contains::Contains;
use crate::core::string::length::Length;
use crate::core::string::regexp::Regexp;
use crate::core::string::split::Split;

use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["string"]));
    pkg.register_function("length".into(), Length);
    pkg.register_function("count".into(), Length);
    pkg.register_function("regexp".into(), Regexp);
    pkg.register_function("prepend".into(), Concat::Prepend);
    pkg.register_function("append".into(), Concat::Append);
    pkg.register_function("contains".into(), Contains);
    pkg.register_function("split".into(), Split);
    pkg
}
