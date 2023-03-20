use crate::core::lang::and::And;
use crate::core::lang::chain::Chain;
use crate::core::lang::map::Map;
use crate::core::lang::map_or_null::MapOrNull;
use crate::core::lang::not::Not;
use crate::core::lang::or::Or;
use crate::core::lang::refine::Refine;
use crate::core::lang::traverse::Traverse;

use crate::package::Package;

use crate::runtime::PackagePath;

mod and;
mod chain;
mod map;
mod map_or_null;
mod not;
mod or;
mod refine;
mod traverse;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["lang"]));
    pkg.register_function("and".into(), And);
    pkg.register_function("or".into(), Or);
    pkg.register_function("refine".into(), Refine);
    pkg.register_function("traverse".into(), Traverse);
    pkg.register_function("chain".into(), Chain);
    pkg.register_function("not".into(), Not);
    pkg.register_function("map".into(), Map);
    pkg.register_function("map-or-null".into(), MapOrNull);
    pkg
}
