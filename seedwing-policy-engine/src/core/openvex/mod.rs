use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::package::Package;
use crate::runtime::rationale::Rationale;
use crate::runtime::{Output, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::RuntimeValue;

use chrono::Utc;

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

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
        _ctx: &'v mut EvalContext,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            match input.as_ref() {
                RuntimeValue::List(items) => {
                    let mut result: Vec<OpenVex> = Vec::new();
                    for item in items.iter() {
                        match serde_json::from_value::<OsvResponse>(item.as_json()) {
                            Ok(osv) => {
                                if let Some(vex) = osv2vex(osv) {
                                    result.push(vex);
                                }
                            }
                            Err(e) => {
                                log::warn!("Error looking up {:?}", e);
                                return Ok(Output::None.into());
                            }
                        }
                    }

                    let vex = merge(result);
                    let json: serde_json::Value = serde_json::to_value(vex).unwrap();
                    Ok(Output::Transform(Arc::new(json.into())).into())
                }
                RuntimeValue::Object(osv) => {
                    match serde_json::from_value::<OsvResponse>(osv.as_json()) {
                        Ok(osv) => {
                            if let Some(vex) = osv2vex(osv) {
                                let json: serde_json::Value = serde_json::to_value(vex).unwrap();
                                Ok(Output::Transform(Arc::new(json.into())).into())
                            } else {
                                Ok(Output::None.into())
                            }
                        }
                        Err(e) => {
                            log::warn!("Error looking up {:?}", e);
                            Ok(Output::None.into())
                        }
                    }
                }
                _v => {
                    let msg = "input is neither a Object nor a List";
                    Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
                }
            }
        })
    }
}

fn merge(mut vexes: Vec<OpenVex>) -> Option<OpenVex> {
    if vexes.is_empty() {
        None
    } else {
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
        for v in vexes.drain(..) {
            vex.statements.extend(v.statements);
        }
        Some(vex)
    }
}

const VERSION: AtomicU64 = AtomicU64::new(1);
fn osv2vex(osv: OsvResponse) -> Option<OpenVex> {
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
            vuln_description: vuln.summary.clone(),
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

    if vex.statements.is_empty() {
        None
    } else {
        Some(vex)
    }
}
