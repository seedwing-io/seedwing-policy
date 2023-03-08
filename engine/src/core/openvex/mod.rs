use crate::package::Package;
use crate::runtime::PackagePath;
use chrono::Utc;
use std::sync::atomic::{AtomicU64, Ordering};

use openvex::*;
mod csaf;
mod merge;
mod osv;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["openvex"]));
    pkg.register_source("".into(), include_str!("openvex.dog"));
    pkg.register_function("from-osv".into(), osv::FromOsv);
    pkg.register_function("from-csaf".into(), csaf::FromCsaf);
    pkg.register_function("merge".into(), merge::Merge);
    pkg
}

pub(crate) fn merge(mut vexes: Vec<OpenVex>) -> Option<OpenVex> {
    if vexes.is_empty() {
        None
    } else {
        let mut vex = openvex();
        for v in vexes.drain(..) {
            vex.statements.extend(v.statements);
        }
        Some(vex)
    }
}

static VERSION: AtomicU64 = AtomicU64::new(1);

pub(crate) fn openvex() -> OpenVex {
    OpenVex {
        metadata: Metadata {
            context: "https://openvex.dev/ns".to_string(),
            id: format!(
                "https://seedwing.io/ROOT/generated/{}",
                uuid::Uuid::new_v4()
            ),
            author: "Seedwing Policy Engine".to_string(),
            role: "Document Creator".to_string(),
            timestamp: Some(Utc::now()),
            version: format!("{}", VERSION.fetch_add(1, Ordering::Relaxed)),
            tooling: Some("Seedwing Policy Engine".to_string()),
            supplier: Some("seedwing.io".to_string()),
        },
        statements: Vec::new(),
    }
}
