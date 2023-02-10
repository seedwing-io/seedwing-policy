use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::package::Package;
use crate::runtime::rationale::Rationale;
use crate::runtime::{Output, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::{RationaleResult, RuntimeValue};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Utc};
use futures_util::future::join_all;
use futures_util::{FutureExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use sigstore::rekor::apis::configuration::Configuration;
use sigstore::rekor::apis::{entries_api, index_api};
use sigstore::rekor::models::SearchIndex;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

mod openvex;

use super::osv::osv::*;
use openvex::*;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["openvex"]));
    pkg.register_source("".into(), include_str!("openvex.dog"));
    pkg.register_function("from-osv".into(), FromOsv);
    pkg
}

#[derive(Debug)]
pub struct FromOsv;

const DOCUMENTATION: &str = include_str!("from-osv.adoc");

impl Function for FromOsv {
    fn order(&self) -> u8 {
        132
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            match input.as_ref() {
                RuntimeValue::Object(osv) => {
                    match serde_json::from_value::<OsvResponse>(osv.as_json()) {
                        Ok(osv) => {
                            let vex = osv2vex(osv);
                            let json: serde_json::Value = serde_json::to_value(vex).unwrap();
                            Ok(Output::Transform(Arc::new(json.into())).into())
                        }
                        Err(e) => {
                            log::warn!("Error looking up {:?}", e);
                            Ok(Output::None.into())
                        }
                    }
                }
                v => {
                    let msg = "input is neither a Object nor a List";
                    Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
                }
            }
        })
    }
}

fn osv2vex(osv: OsvResponse) -> OpenVex {
    const VERSION: AtomicU64 = AtomicU64::new(1);
    let mut vex = OpenVex {
        metadata: Metadata {
            context: "https://openvex.dev/ns".to_string(),
            id: format!(
                "https://seedwing.io/docs/generated/{}",
                uuid::Uuid::new_v4().to_string()
            ),
            author: "Seedwing Policy Engine".to_string(),
            role: "Document Creator".to_string(),
            timestamp: Some(Utc::now()),
            version: format!("{}", VERSION.fetch_add(1, Ordering::Relaxed)),
            tooling: Some("Seedwing Policy Engine".to_string()),
            supplier: Some("seedwing.io".to_string()),
        },
        statements: Vec::new(),
    };

    for vuln in osv.vulns.iter() {
        let mut products = HashSet::new();
        let status = Status::Affected;
        let justification = None;

        for affected in vuln.affected.iter() {
            if let Some(purl) = &affected.package.purl {
                for version in affected.versions.iter() {
                    products.insert(format!("{}@{}", purl, version));
                }
                if products.is_empty() {
                    products.insert(purl.clone());
                }
            }
        }
        let statement = Statement {
            vulnerability: Some(vuln.id.clone()),
            vuln_description: Some(vuln.summary.clone()),
            timestamp: Some(vuln.modified),
            products: products.drain().collect(),
            subcomponents: Vec::new(),
            status,
            status_notes: Some(format!(
                "Open Source Vulnerabilities (OSV) found vulnerabilities"
            )),
            justification,
            impact_statement: None,
            action_statement: Some(format!(
                "Review {} for details on the appropriate action",
                vuln.id.clone()
            )),
            action_statement_timestamp: Some(Utc::now()),
        };
        vex.statements.push(statement);
    }

    vex
}
