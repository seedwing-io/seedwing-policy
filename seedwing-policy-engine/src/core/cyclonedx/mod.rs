use crate::package::Package;
use crate::runtime::PackagePath;

//mod ovs;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["cyclonedx"]));
    pkg.register_source("v1_4".into(), include_str!("v1_4.dog"));
    //pkg.register_source("v1_4/structure".into(), include_str!("v1_4/v1_4.dog"));
    pkg.register_source("hash".into(), include_str!("hash.dog"));
    //    pkg.register_function("Ovs".into(), ovs::Ovs2Vex);
    pkg
}
