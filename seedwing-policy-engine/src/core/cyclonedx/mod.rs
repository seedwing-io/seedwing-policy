use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, EvalContext};
use crate::package::Package;
use crate::runtime::PackagePath;
use crate::runtime::World;
use crate::runtime::{Output, RuntimeError};
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

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["cyclonedx"]));
    pkg.register_source("v1_4".into(), include_str!("v1_4.dog"));
    //pkg.register_source("v1_4/structure".into(), include_str!("v1_4/v1_4.dog"));
    pkg.register_source("hash".into(), include_str!("hash.dog"));
    pkg.register_function("component-purls".into(), ComponentPurls);
    pkg
}

#[derive(Debug)]
pub struct ComponentPurls;

const DOCUMENTATION: &str = include_str!("component-purls.adoc");

impl Function for ComponentPurls {
    fn order(&self) -> u8 {
        128
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
            match input.as_json() {
                serde_json::Value::Object(o) => {
                    let mut purls = Vec::new();
                    if let Some(serde_json::Value::Array(components)) = o.get("components") {
                        for component in components.iter() {
                            if let serde_json::Value::Object(c) = component {
                                if let Some(serde_json::Value::String(s)) = c.get("purl") {
                                    purls.push(Arc::new(RuntimeValue::String(s.clone())));
                                }
                            }
                        }
                    }
                    Ok(Output::Transform(Arc::new(RuntimeValue::List(purls))).into())
                }
                _ => Ok(Output::None.into()),
            }
        })
    }
}
