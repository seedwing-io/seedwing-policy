use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::Bindings;
use crate::lang::PackagePath;
use crate::package::Package;
use crate::runtime::{Output, RuntimeError};
use crate::value::{RationaleResult, RuntimeValue};
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

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["sigstore"]));
    pkg.register_function("SHA256".into(), SHA256);
    pkg
}

#[derive(Debug)]
pub struct SHA256;

const DOCUMENTATION: &str = include_str!("SHA256.adoc");

impl Function for SHA256 {
    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn call<'v>(
        &'v self,
        input: Rc<RuntimeValue>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            let input = (*input).borrow();
            if let Some(digest) = input.try_get_string() {
                let configuration = Configuration::default();
                let query = SearchIndex {
                    email: None,
                    public_key: None,
                    hash: Some(digest),
                };
                let uuid_vec = index_api::search_index(&configuration, query).await;
                if let Ok(uuid_vec) = uuid_vec {
                    let mut transform: Vec<RuntimeValue> = Vec::new();
                    for uuid in uuid_vec.iter() {
                        let entry = entries_api::get_log_entry_by_uuid(&configuration, uuid).await;
                        if let Ok(entry) = entry {
                            let body = base64::decode(entry.body);
                            if let Ok(body) = body {
                                let body: Result<serde_json::Value, _> =
                                    serde_json::from_slice(&*body);
                                if let Ok(body) = body {
                                    let value = (&body).into();
                                    transform.push(value)
                                }
                            }
                        }
                    }

                    Ok(Output::Transform(Rc::new(transform.into())).into())
                } else {
                    Ok(Output::None.into())
                }
            } else {
                Ok(Output::None.into())
            }
        })
    }
}
