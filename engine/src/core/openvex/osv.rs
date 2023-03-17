use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::rationale::Rationale;
use crate::runtime::World;
use crate::runtime::{EvalContext, Output, RuntimeError};
use crate::value::RuntimeValue;
use chrono::Utc;

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

use super::super::osv::client::*;
use crate::lang::PatternMeta;
use openvex::*;

#[derive(Debug)]
pub struct FromOsv;

const DOCUMENTATION: &str = include_str!("from-osv.adoc");

impl Function for FromOsv {
    fn order(&self) -> u8 {
        132
    }
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: Some(DOCUMENTATION.into()),
            ..Default::default()
        }
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
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
                                result.push(osv2vex(osv));
                            }
                            Err(e) => {
                                log::warn!("Error looking up {:?}", e);
                                return Ok(Output::None.into());
                            }
                        }
                    }

                    let vex = super::merge(result);
                    let json: serde_json::Value = serde_json::to_value(vex).unwrap();
                    Ok(Output::Transform(Arc::new(json.into())).into())
                }
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
                _v => {
                    let msg = "input is neither a Object nor a List";
                    Ok((Output::None, Rationale::InvalidArgument(msg.into())).into())
                }
            }
        })
    }
}

fn osv2vex(osv: OsvResponse) -> OpenVex {
    let mut vex = super::openvex();

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
            status_notes: Some("Open Source Vulnerabilities (OSV) found vulnerabilities".into()),
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
