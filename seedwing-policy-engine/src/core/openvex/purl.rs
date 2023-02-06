use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::package::Package;
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
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::Arc;

use super::openvex::*;
use super::osv::*;

#[derive(Debug)]
pub struct FromPurl;

const DOCUMENTATION: &str = include_str!("FromPurl.adoc");

impl Function for FromPurl {
    fn order(&self) -> u8 {
        // Reaching out to the network
        200
    }
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let client = OsvClient::new();

            use serde_json::Value as JsonValue;
            let input = input.as_json();
            log::info!("Input is :{:?}", input);
            match (
                input.get("name"),
                input.get("namespace"),
                input.get("type"),
                input.get("version"),
            ) {
                (
                    Some(JsonValue::String(name)),
                    Some(JsonValue::String(namespace)),
                    Some(JsonValue::String(ecosystem)),
                    Some(JsonValue::String(version)),
                ) => match client
                    .query(&ecosystem, &format!("{}:{}", namespace, name), &version)
                    .await
                {
                    Ok(transform) => {
                        let vex = osv2vex(transform);
                        let json: serde_json::Value = serde_json::to_value(vex).unwrap();
                        return Ok(Output::Transform(Rc::new(json.into())).into());
                    }
                    Err(e) => {
                        log::warn!("Error looking up {:?}", e);
                        Ok(Output::None.into())
                    }
                },
                _ => Ok(Output::None.into()),
            }
        })
    }
}

fn osv2vex(osv: OsvResponse) -> OpenVex {
    let mut vex = OpenVex {
        metadata: Metadata {
            context: "https://github.com/seedwing-io/seedwing-policy".to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            author: "Seedwing Policy Engine".to_string(),
            role: "Document Creator".to_string(),
            timestamp: Some(Utc::now()),
            // TODO: Should we based this on some global atomic?
            version: "1".to_string(),
            tooling: Some("Seedwing Policy Engine".to_string()),
            supplier: Some("seedwing.io".to_string()),
        },
        statements: Vec::new(),
    };

    for vuln in osv.vulns.iter() {
        let mut products = Vec::new();
        let status = Status::Affected;
        let justification = None;

        for affected in vuln.affected.iter() {
            if let Some(purl) = &affected.package.purl {
                products.push(purl.clone());
            }
            for range in affected.ranges.iter() {}
        }
        let statement = Statement {
            vulnerability: Some(vuln.id.clone()),
            vuln_description: Some(vuln.summary.clone()),
            timestamp: Some(vuln.modified),
            products,
            subcomponents: Vec::new(),
            status,
            status_notes: None,
            justification,
            impact_statement: None,
            action_statement: None,
            action_statement_timestamp: None,
        };
        vex.statements.push(statement);
    }

    vex
}
/*
curl -X POST -d \
          '{"version": "2.4.1", "package": {"name": "jinja2", "ecosystem": "PyPI"}}' \
          "https://api.osv.dev/v1/query"*/
