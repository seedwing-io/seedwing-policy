mod eval;
mod remote;

use crate::{
    core::external::{eval::Eval, remote::Remote},
    package::Package,
    runtime::PackagePath,
};

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["external"]))
        .with_documentation(r#"Work with remote policy servers"#);

    pkg.register_function("eval".into(), Eval);
    pkg.register_function("remote".into(), Remote::new());
    pkg
}
