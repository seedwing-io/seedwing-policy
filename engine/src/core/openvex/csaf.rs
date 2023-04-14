use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::runtime::rationale::Rationale;
use crate::runtime::World;
use crate::runtime::{EvalContext, Output, RuntimeError};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

use crate::lang::{PatternMeta, Severity};
use csaf::{vulnerability::FlagLabel, *};
use openvex::*;

#[derive(Debug)]
pub struct FromCsaf;

const DOCUMENTATION: &str = include_str!("from-csaf.adoc");

impl Function for FromCsaf {
    fn order(&self) -> u8 {
        132
    }
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
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
                        match serde_json::from_value::<Csaf>(item.as_json()) {
                            Ok(csaf) => {
                                result.push(csaf2vex(csaf));
                            }
                            Err(e) => {
                                log::warn!("Error looking up {e:?}");
                                return Ok(Severity::Error.into());
                            }
                        }
                    }

                    let vex = super::merge(result);
                    let json: serde_json::Value = serde_json::to_value(vex).unwrap();
                    Ok(Output::Transform(Arc::new(json.into())).into())
                }
                RuntimeValue::Object(csaf) => {
                    match serde_json::from_value::<Csaf>(csaf.as_json()) {
                        Ok(csaf) => {
                            let vex = csaf2vex(csaf);
                            let json: serde_json::Value = serde_json::to_value(vex).unwrap();
                            Ok(Output::Transform(Arc::new(json.into())).into())
                        }
                        Err(e) => {
                            log::warn!("Error looking up {e:?}");
                            Ok(Severity::Error.into())
                        }
                    }
                }
                _v => {
                    let msg = "input is neither a Object nor a List";
                    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
                }
            }
        })
    }
}

fn csaf2vex(csaf: Csaf) -> OpenVex {
    let mut vex = super::openvex();
    if let Some(vuln) = csaf.vulnerabilities {
        for vuln in vuln.iter() {
            let vulnerability = &vuln.cve;
            let vuln_description = &vuln.title;
            let timestamp = vuln.release_date;
            if let Some(pstatus) = &vuln.product_status {
                if let Some(s) = &pstatus.fixed {
                    let mut products = Vec::new();
                    for p in s.iter() {
                        products.push(p.0.clone());
                    }
                    let statement = Statement {
                        vulnerability: vulnerability.clone(),
                        vuln_description: vuln_description.clone(),
                        timestamp,
                        products,
                        subcomponents: vec![],
                        status: Status::Fixed,
                        status_notes: None,
                        justification: None,
                        impact_statement: None,
                        action_statement: None,
                        action_statement_timestamp: None,
                    };
                    vex.statements.push(statement);
                }

                if let Some(s) = &pstatus.known_affected {
                    let mut products = Vec::new();
                    for p in s.iter() {
                        products.push(p.0.clone());
                    }

                    let mut action_statement = None;
                    let mut action_statement_timestamp = None;
                    if let Some(remediations) = &vuln.remediations {
                        for remediation in remediations.iter() {
                            action_statement.replace(remediation.details.clone());
                            action_statement_timestamp = remediation.date;
                        }
                    }
                    let statement = Statement {
                        vulnerability: vulnerability.clone(),
                        vuln_description: vuln_description.clone(),
                        timestamp,
                        products,
                        subcomponents: vec![],
                        status: Status::Affected,
                        status_notes: None,
                        justification: None,
                        impact_statement: None,
                        action_statement,
                        action_statement_timestamp,
                    };
                    vex.statements.push(statement);
                }

                if let Some(s) = &pstatus.known_not_affected {
                    let mut products = Vec::new();
                    for p in s.iter() {
                        products.push(p.0.clone());
                    }

                    let mut justification = None;
                    if let Some(flags) = &vuln.flags {
                        for flag in flags.iter() {
                            justification.replace(flag2justification(&flag.label));
                        }
                    }

                    let statement = Statement {
                        vulnerability: vulnerability.clone(),
                        vuln_description: vuln_description.clone(),
                        timestamp,
                        products,
                        subcomponents: vec![],
                        status: Status::NotAffected,
                        status_notes: None,
                        justification,
                        impact_statement: None,
                        action_statement: None,
                        action_statement_timestamp: None,
                    };
                    vex.statements.push(statement);
                }

                if let Some(s) = &pstatus.under_investigation {
                    let mut products = Vec::new();
                    for p in s.iter() {
                        products.push(p.0.clone());
                    }
                    let statement = Statement {
                        vulnerability: vulnerability.clone(),
                        vuln_description: vuln_description.clone(),
                        timestamp,
                        products,
                        subcomponents: vec![],
                        status: Status::UnderInvestigation,
                        status_notes: None,
                        justification: None,
                        impact_statement: None,
                        action_statement: None,
                        action_statement_timestamp: None,
                    };
                    vex.statements.push(statement);
                }
            }
        }
    }

    vex
}

fn flag2justification(flag: &FlagLabel) -> Justification {
    match flag {
        FlagLabel::ComponentNotPresent => Justification::ComponentNotPresent,
        FlagLabel::InlineMitigationsAlreadyExist => Justification::InlineMitigationsAlreadyExist,
        FlagLabel::VulnerableCodeCannotBeControlledByAdversary => {
            Justification::VulnerableCodeCannotBeControlledByAdversary
        }
        FlagLabel::VulnerableCodeNotInExecutePath => Justification::VulnerableCodeNotInExecutePath,
        FlagLabel::VulnerableCodeNotPresent => Justification::VulnerableCodeNotPresent,
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_satisfied;
    use crate::runtime::testutil::test_pattern;

    #[tokio::test]
    async fn test_from_csaf() {
        let input = include_str!("../csaf/rhba-2023_0564.json");
        let json: serde_json::Value = serde_json::from_str(input).unwrap();
        let result = test_pattern(r#"openvex::from-csaf"#, json).await;
        assert_satisfied!(result);
    }
}
