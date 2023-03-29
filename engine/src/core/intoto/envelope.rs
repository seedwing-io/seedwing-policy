use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern};
use crate::lang::{Severity, ValuePattern};
use crate::runtime::rationale::Rationale;
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::Object;
use crate::value::RuntimeValue;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha2::Sha256;
use sigstore::tuf::SigstoreRepository;
use std::path::{Path, PathBuf};
use tokio::task::spawn_blocking;

use crate::lang::PatternMeta;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::str;
use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("verify-envelope.adoc");
const ATTESTERS: &str = "attesters";
const BLOB: &str = "blob";

#[derive(Debug)]
pub struct Verify;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Envelope {
    #[serde(rename = "payloadType")]
    payload_type: String,
    payload: String,
    signatures: Vec<Signature>,
}

impl Envelope {
    fn payload_from_base64(&self) -> Result<String, anyhow::Error> {
        match general_purpose::STANDARD.decode(&self.payload) {
            Ok(value) => Ok(String::from_utf8(value).unwrap()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Signature {
    cert: String,
    #[serde(rename = "kid")]
    keyid: Option<String>,
    #[serde(rename = "sig")]
    value: String,
}

impl Signature {
    fn cert_as_base64(&self) -> String {
        general_purpose::STANDARD.encode(&self.cert)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Statement {
    _type: String,
    #[serde(rename = "subject")]
    subjects: Vec<Subject>,
    #[serde(rename = "predicateType")]
    predicate_type: String,
    predicate: serde_json::Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Subject {
    name: String,
    digest: HashMap<String, String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct VerifyOutput {
    predicate_type: String,
    predicate: serde_json::Value,
    attester_names: Vec<String>,
    artifact_names: Vec<String>,
}

impl Function for Verify {
    fn metadata(&self) -> PatternMeta {
        PatternMeta {
            documentation: DOCUMENTATION.into(),
            ..Default::default()
        }
    }

    fn parameters(&self) -> Vec<String> {
        vec![ATTESTERS.into(), BLOB.into()]
    }

    /// This function follows the validation model specified in:
    /// https://github.com/in-toto/attestation/blob/main/docs/validation.md
    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let serde_json::Value::Object(o) = input.as_json() {
                let envelope: serde_json::Value = o.clone().into();
                let envelope: Envelope = serde_json::from_value(envelope).unwrap();

                if envelope.payload_type != "application/vnd.in-toto+json" {
                    return invalid_type("payloadType", envelope.payload_type);
                }

                let decoded_payload = match envelope.payload_from_base64() {
                    Ok(value) => value,
                    Err(_) => return base64_decode_error("payload"),
                };

                // This is Pre-Authenticated Encoding (PAE) which is what
                // is actually verified (and what is signed by the producer
                // of the signature).
                let pae = pae(&envelope.payload_type, &decoded_payload);
                log::debug!("pae: {}", pae);

                let attesters_map = get_attesters(ATTESTERS, bindings);
                if attesters_map.is_empty() {
                    return missing_attesters();
                }

                let blob = match get_blob(input, bindings, ctx, world).await {
                    Ok(value) => value,
                    Err(_) => return blob_error(),
                };

                let mut attesters_names: Vec<Arc<RuntimeValue>> = Vec::new();
                let mut artifact_names: Vec<Arc<RuntimeValue>> = Vec::new();

                // Fetch from The Update Framework (TUF) repository
                #[cfg(not(target_arch = "wasm32"))]
                let _repo: sigstore::errors::Result<SigstoreRepository> =
                    spawn_blocking(move || {
                        let checkout_dir: Option<PathBuf> = home::home_dir()
                            .as_ref()
                            .map(|h| h.join(".sigstore").join("root").join("targets"));
                        let path: Option<&Path> = checkout_dir.as_deref();
                        log::info!("checkout_dir: {:?}", path);
                        sigstore::tuf::SigstoreRepository::fetch(path)
                    })
                    .await
                    .unwrap();

                let mut verified: Vec<Arc<RuntimeValue>> = Vec::new();
                for sig in envelope.signatures.iter() {
                    let cert_base64 = sig.cert_as_base64();
                    log::debug!("cert_base64: {:?}", cert_base64);
                    log::debug!("sig.keyid: {:?}", sig.keyid);
                    log::debug!("sig.value: {:?}", sig.value);
                    log::debug!("attesters_map: {:?}", attesters_map);

                    for (name, publickey) in &attesters_map {
                        log::info!("name: {}, key: {}", name, publickey);
                        if publickey != &cert_base64 {
                            continue;
                        }

                        match sigstore::cosign::verify_blob(
                            cert_base64.trim(),
                            &sig.value,
                            &pae.clone().into_bytes(),
                        ) {
                            Ok(_) => {
                                attesters_names
                                    .push(Arc::new(RuntimeValue::from(name.to_string())));
                                log::info!("Verified succeeded!");
                                let statement: Statement =
                                    match serde_json::from_str(&decoded_payload) {
                                        Ok(value) => value,
                                        Err(e) => {
                                            log::error!("{:?}", e);
                                            return json_parse_error("payload");
                                        }
                                    };

                                if statement._type != "https://in-toto.io/Statement/v0.1" {
                                    return invalid_type("_type", statement._type);
                                }

                                for subject in statement.subjects {
                                    for (alg, digest) in &subject.digest {
                                        if let Ok(hash) = hash(&blob, alg) {
                                            if &hash == digest {
                                                artifact_names.push(Arc::new(RuntimeValue::from(
                                                    subject.name.to_string(),
                                                )));
                                            }
                                        }
                                    }
                                }

                                let mut output = Object::new();
                                output.set("predicate_type", statement.predicate_type);
                                output.set("predicate", statement.predicate.clone());
                                output.set("attesters_names", attesters_names.clone());
                                output.set("artifact_names", artifact_names.clone());
                                verified.push(Arc::new(RuntimeValue::Object(output)));
                            }
                            Err(e) => {
                                log::error!("verify_blob failed with {:?}", e);
                            }
                        }
                    }
                }
                if !verified.is_empty() {
                    return Ok(Output::Transform(Arc::new(RuntimeValue::List(verified))).into());
                }
            }
            Ok(Severity::Error.into())
        })
    }
}

fn hash(blob: &Vec<u8>, alg: &str) -> Result<String, anyhow::Error> {
    match alg {
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(blob);
            let hash = format!("{:x}", hasher.finalize());
            Ok(hash)
        }
        _ => Err(anyhow::anyhow!("Could not find a hasher for {alg}")),
    }
}

async fn get_blob<'v>(
    input: Arc<RuntimeValue>,
    bindings: &Bindings,
    ctx: &'v EvalContext,
    world: &'v World,
) -> Result<Vec<u8>, anyhow::Error> {
    if let Some(pattern) = bindings.get(BLOB) {
        let result = pattern
            .evaluate(input.clone(), ctx, bindings, world)
            .await?;

        if matches!(result.severity(), Severity::Error) {
            if let Some(octs) = result.output().try_get_octets() {
                return Ok(octs.to_owned());
            }
        }
    }
    Err(anyhow::anyhow!("Could not evaluate blob"))
}

/// Pre-Authenticated Encoding (PAE) for DSSEv1
fn pae(payload_type: &str, payload: &str) -> String {
    let pae = format!(
        "DSSEv1 {} {} {} {}",
        payload_type.len(),
        payload_type,
        payload.len(),
        payload,
    );
    pae
}

fn get_attesters(param: &str, bindings: &Bindings) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(pattern) = bindings.get(param) {
        if let InnerPattern::List(list) = pattern.inner() {
            for item in list {
                if let InnerPattern::Object(p) = item.inner() {
                    let mut values = [Default::default(), Default::default()];
                    for (i, field) in p.fields().iter().enumerate() {
                        if let InnerPattern::Const(ValuePattern::String(value)) = field.ty().inner()
                        {
                            values[i] = value.to_string();
                        };
                    }
                    map.insert(values[0].to_owned(), values[1].to_owned());
                }
            }
        }
    }
    map
}

fn base64_decode_error(field: impl Into<String>) -> Result<FunctionEvaluationResult, RuntimeError> {
    let msg = format!("Could not decode {} field to base64", field.into());
    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
}

fn json_parse_error(field: impl Into<String>) -> Result<FunctionEvaluationResult, RuntimeError> {
    let msg = format!("Could not parse {}", field.into());
    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
}

fn missing_attesters() -> Result<FunctionEvaluationResult, RuntimeError> {
    let msg = "At least one attester must be provided in the attesters parameter";
    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
}

fn blob_error() -> Result<FunctionEvaluationResult, RuntimeError> {
    let msg = "Blob could not be parsed. Please check if a data source directory was set.";
    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
}

fn invalid_type(
    field: impl Into<String>,
    value: impl Into<String>,
) -> Result<FunctionEvaluationResult, RuntimeError> {
    let msg = format!("invalid {} specified {}", field.into(), value.into());
    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        assert_not_satisfied, assert_satisfied, runtime::testutil::test_data_dir,
        runtime::testutil::test_patterns,
    };
    use serde_json::json;
    use std::fs;

    #[actix_rt::test]
    async fn verify_envelope() {
        let input_str = fs::read_to_string(
            test_data_dir()
                .join("intoto")
                .join("example-intoto-envelope.json"),
        )
        .unwrap();
        let input_json: serde_json::Value = serde_json::from_str(&input_str).unwrap();
        let result = test_patterns(
            r#"
            pattern blob = *data::from<"intoto/binary-linux-amd64">

            pattern attesters = [
              {name: "dan", public_key: "LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0tCk1JSUR3RENDQTBhZ0F3SUJBZ0lVTEpaajZlQVp0c1dkSUhGcktnK00rTFZkTkEwd0NnWUlLb1pJemowRUF3TXcKTnpFVk1CTUdBMVVFQ2hNTWMybG5jM1J2Y21VdVpHVjJNUjR3SEFZRFZRUURFeFZ6YVdkemRHOXlaUzFwYm5SbApjbTFsWkdsaGRHVXdIaGNOTWpNd016RTBNVEF5TlRBMVdoY05Nak13TXpFME1UQXpOVEExV2pBQU1Ga3dFd1lICktvWkl6ajBDQVFZSUtvWkl6ajBEQVFjRFFnQUVtSUF2WFZMVGg2NkUzV2RXUkZac1ZTSE9VQ2swbUwrazRLSXYKYU4zOWhHekhncHozalp2Ylp3NnhTaHJidVZYVW4wMUFQck0vUWh0YVZhMWJtZUJLV0tPQ0FtVXdnZ0poTUE0RwpBMVVkRHdFQi93UUVBd0lIZ0RBVEJnTlZIU1VFRERBS0JnZ3JCZ0VGQlFjREF6QWRCZ05WSFE0RUZnUVVkbkhyCjlKdFFlQlFHVnhtU0JkWHFBMnhDVXlVd0h3WURWUjBqQkJnd0ZvQVUzOVBwejFZa0VaYjVxTmpwS0ZXaXhpNFkKWkQ4d2ZRWURWUjBSQVFIL0JITXdjWVp2YUhSMGNITTZMeTluYVhSb2RXSXVZMjl0TDNOc2MyRXRabkpoYldWMwpiM0pyTDNOc2MyRXRaMmwwYUhWaUxXZGxibVZ5WVhSdmNpOHVaMmwwYUhWaUwzZHZjbXRtYkc5M2N5OWlkV2xzClpHVnlYMmR2WDNOc2MyRXpMbmx0YkVCeVpXWnpMM1JoWjNNdmRqRXVOUzR3TURrR0Npc0dBUVFCZzc4d0FRRUUKSzJoMGRIQnpPaTh2ZEc5clpXNHVZV04wYVc5dWN5NW5hWFJvZFdKMWMyVnlZMjl1ZEdWdWRDNWpiMjB3RWdZSwpLd1lCQkFHRHZ6QUJBZ1FFY0hWemFEQTJCZ29yQmdFRUFZTy9NQUVEQkNoaU5qQXhZek13WWpNeFl6UmxPRE14CllqRmhPRFF4T0daa01Ua3paakEwWXpJM05XUXlNVEJqTUJNR0Npc0dBUVFCZzc4d0FRUUVCVWR2SUVOSk1ERUcKQ2lzR0FRUUJnNzh3QVFVRUkzTmxaV1IzYVc1bkxXbHZMM05sWldSM2FXNW5MV2R2YkdGdVp5MWxlR0Z0Y0d4bApNQjhHQ2lzR0FRUUJnNzh3QVFZRUVYSmxabk12ZEdGbmN5OTJNQzR4TGpFMU1JR0tCZ29yQmdFRUFkWjVBZ1FDCkJId0VlZ0I0QUhZQTNUMHdhc2JIRVRKakdSNGNtV2MzQXFKS1hyamVQSzMvaDRweWdDOHA3bzRBQUFHRzM2YnkKSmdBQUJBTUFSekJGQWlFQTlyYnVNRDNoeHFkbTRCU1kxNmNncGlFMCtabWZITk9FbjhrblJqenB3WkVDSURnaAo2a1g0d005ZDVJUGlsdkZ6bjJ4KytJU0tYaU9LdmZyS24xa0tUaFR3TUFvR0NDcUdTTTQ5QkFNREEyZ0FNR1VDCk1FTy9qeG11aVBpUGRmVkREY1hBRVowSFRSVXA5V3Bjc2Y4dlhkdTFqODRVd291ZzUzaXZsdW1Yb0ZxN2hlSzEKdGdJeEFQQ29sOTk3QTgrTnFLVWllcmw5RGFFd2hBcG5HWlVTNXJ2MS9TcWpwbEpJSGhFTHFUMzZoNjR5dzl1QwprUDhlRGc9PQotLS0tLUVORCBDRVJUSUZJQ0FURS0tLS0tCg=="}
            ]

            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            RuntimeValue::from(&input_json)
        ).await;
        assert_satisfied!(result);

        let output = result.output().unwrap().as_json();
        assert!(output.is_array());
        let output = &output[0];

        let input_payload: serde_json::Value = payload_as_json(&input_json);
        assert_eq!(
            output.get("predicate_type").unwrap(),
            input_payload.get("predicateType").unwrap(),
        );
        assert_eq!(
            output.get("predicate").unwrap(),
            input_payload.get("predicate").unwrap(),
        );
        assert_eq!(output.get("attesters_names").unwrap()[0], "dan");
        assert_eq!(
            output.get("artifact_names").unwrap()[0],
            "binary-linux-amd64"
        );
    }

    #[actix_rt::test]
    async fn verify_envelope_invalid_attesters() {
        let input_str = fs::read_to_string(
            test_data_dir()
                .join("intoto")
                .join("example-intoto-envelope.json"),
        )
        .unwrap();
        let input_json: serde_json::Value = serde_json::from_str(&input_str).unwrap();
        let result = test_patterns(
            r#"
            pattern blob = *data::from<"intoto/binary-linux-amd64">

            pattern attesters = [
              {name: "dan", public_key: "dummy-value"}
            ]

            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            RuntimeValue::from(&input_json),
        )
        .await;
        assert_not_satisfied!(result);
    }

    #[actix_rt::test]
    async fn verify_envelope_invalid_payload_type() {
        let input = json!({
           "payloadType": "application/vnd.in-typo+json",
           "payload": "dummy",
           "signatures": [{"sig": "dummy", "cert": "anything"}]
        });
        let value = RuntimeValue::from(input);
        let result = test_patterns(
            r#"
            pattern blob = *data::from<"binary-linux-amd64">
            pattern attesters = [{name: "dan", public_key: "bogus"}]
            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            value,
        )
        .await;
        assert_not_satisfied!(result);

        if let Rationale::Function(_, out, _) = result.rationale() {
            if let Rationale::InvalidArgument(msg) = &**(out.as_ref().unwrap()) {
                assert_eq!(
                    msg,
                    "invalid payloadType specified application/vnd.in-typo+json"
                );
            }
        }
    }

    fn payload_as_json(input: &serde_json::Value) -> serde_json::Value {
        let envelope: Envelope = serde_json::from_value(input.clone()).unwrap();
        let payload_base64 = envelope.payload_from_base64().unwrap();
        let payload: serde_json::Value = serde_json::from_str(&payload_base64).unwrap();
        payload
    }
}
