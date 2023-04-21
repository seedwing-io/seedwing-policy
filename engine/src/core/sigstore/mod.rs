use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::package::Package;
use crate::runtime::{ExecutionContext, Output, RuntimeError};
use crate::runtime::{PackagePath, World};
use crate::value::RuntimeValue;
use futures_util::future::join_all;
use futures_util::FutureExt;
use sigstore::rekor::apis::configuration::Configuration;
use sigstore::rekor::apis::{entries_api, index_api};
use sigstore::rekor::models::SearchIndex;
use std::borrow::Borrow;

use std::future::Future;
use std::pin::Pin;

use crate::lang::{PatternMeta, Severity};
use std::sync::Arc;

mod verify;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["sigstore"]));
    pkg.register_function("sha256".into(), SHA256);
    pkg.register_function("verify-blob".into(), verify::VerifyBlob);
    pkg
}

#[derive(Debug)]
pub struct SHA256;

const DOCUMENTATION: &str = include_str!("sha256.adoc");

impl Function for SHA256 {
    fn order(&self) -> u8 {
        // Reaching out to the network
        200
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
        _ctx: ExecutionContext<'v>,
        _bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let input = (*input).borrow();
            if let Some(digest) = input.try_get_str() {
                let configuration = Configuration::default();
                let query = SearchIndex {
                    email: None,
                    public_key: None,
                    hash: Some(digest.to_string()),
                };
                let uuid_vec = index_api::search_index(&configuration, query).await;
                if let Ok(uuid_vec) = uuid_vec {
                    let handles = uuid_vec.iter().map(|uuid| {
                        entries_api::get_log_entry_by_uuid(&configuration, uuid.as_str()).map(
                            |entry| {
                                if let Ok(entry) = entry {
                                    let body: Result<serde_json::Value, _> =
                                        serde_json::to_value(entry.body);
                                    if let Ok(body) = body {
                                        let value: RuntimeValue = body.into();
                                        return Some(value);
                                    }
                                }

                                None
                            },
                        )
                    });

                    let joined = join_all(handles).await;
                    let transform: Vec<RuntimeValue> = joined.into_iter().flatten().collect();

                    Ok(Output::Transform(Arc::new(transform.into())).into())
                } else {
                    Ok(Severity::Error.into())
                }
            } else {
                Ok(Severity::Error.into())
            }
        })
    }
}
