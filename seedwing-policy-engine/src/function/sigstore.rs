use std::future::Future;
use std::pin::Pin;
use std::str::from_utf8;
use sigstore::rekor::apis::configuration::Configuration;
use sigstore::rekor::apis::{entries_api, index_api};
use sigstore::rekor::models::SearchIndex;
use crate::function::{Function, FunctionPackage};
use crate::value::Value;

pub fn package() -> FunctionPackage {
    let mut pkg = FunctionPackage::new();
    pkg.register("SHA256".into(), Sha256);
    pkg
}

#[derive(Debug)]
pub struct Sha256;

impl Function for Sha256 {
    fn call<'v>(&'v self, input: &'v mut Value) -> Pin<Box<dyn Future<Output=Result<Value, ()>> + 'v>> {
        Box::pin(
            async move {
                if let Some(digest) = input.try_get_string() {
                    let configuration = Configuration::default();
                    let query = SearchIndex {
                        email: None,
                        public_key: None,
                        hash: Some(digest),
                    };
                    let uuid_vec = index_api::search_index(&configuration, query).await;
                    if let Ok(uuid_vec) = uuid_vec {
                        let mut transform: Vec<Value> = Vec::new();
                        for uuid in uuid_vec.iter() {
                            let entry =
                                entries_api::get_log_entry_by_uuid(&configuration, uuid).await;
                            if let Ok(entry) = entry {
                                let body = base64::decode(entry.body);
                                if let Ok(body) = body {
                                    let body: Result<serde_json::Value, _> = serde_json::from_slice(&*body);
                                    if let Ok(body) = body {
                                        println!("{:?}", body);
                                        let value = (&body).into();
                                        println!("OKAY OKAY OKAY");
                                        //return Ok(value);
                                        transform.push(value)
                                    }
                                }
                            }
                        }

                        return Ok(transform.into())
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            }
        )
    }
}