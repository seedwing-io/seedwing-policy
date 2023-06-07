use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern};
use crate::lang::{Severity, ValuePattern};
use crate::runtime::rationale::Rationale;
use crate::runtime::{ExecutionContext, Output, RuntimeError, World};
use crate::value::Object;
use crate::value::RuntimeValue;
use anyhow::Result;
use base64::engine::{general_purpose::STANDARD as BASE64_STD_ENGINE, Engine as _};
use in_toto::crypto::PublicKey;
use in_toto::crypto::{KeyType, SignatureScheme};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha2::Sha256;
use sha2::Sha512;
use sigstore::cosign::client::Client as Cosign;
use sigstore::cosign::CosignCapabilities;
use sigstore::errors::SigstoreError;
use sigstore::rekor::apis::{configuration::Configuration, entries_api, index_api};
use sigstore::rekor::models::log_entry::Body;
use sigstore::rekor::models::SearchIndex;
use sigstore::tuf::SigstoreRepository;
use ssh_key::public::{EcdsaPublicKey, KeyData};
use ssh_key::sec1::{consts::U32, EncodedPoint};
use ssh_key::HashAlg;
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
        match BASE64_STD_ENGINE.decode(&self.payload) {
            Ok(value) => Ok(String::from_utf8(value).unwrap()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Signature {
    cert: Option<String>,
    #[serde(rename = "keyid")]
    keyid: Option<String>,
    #[serde(rename = "sig")]
    value: String,
}

impl Signature {
    fn cert_as_base64(&self) -> Option<String> {
        if let Some(cert) = &self.cert {
            let encoded = BASE64_STD_ENGINE.encode(cert);
            return Some(encoded);
        }
        None
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

impl Function for Verify {
    fn order(&self) -> u8 {
        255
    }

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
        ctx: ExecutionContext<'v>,
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

                let Ok(decoded_payload) = envelope.payload_from_base64() else {
                    return base64_decode_error("payload");
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

                let Ok(blob) = get_blob(&input, bindings, ctx, world).await else {
                    return blob_error();
                };

                // Fetch from The Update Framework (TUF) repository
                #[cfg(not(target_arch = "wasm32"))]
                let _repo: sigstore::errors::Result<SigstoreRepository> =
                    spawn_blocking(move || {
                        let checkout_dir: Option<PathBuf> = home::home_dir()
                            .as_ref()
                            .map(|h| h.join(".sigstore").join("root").join("targets"));
                        let path: Option<&Path> = checkout_dir.as_deref();
                        log::debug!("sigstore tuf checkout_dir: {:?}", path);
                        sigstore::tuf::SigstoreRepository::fetch(path)
                    })
                    .await
                    .unwrap();

                let mut verified: Vec<Arc<RuntimeValue>> = Vec::new();
                for sig in envelope.signatures.iter() {
                    log::debug!("sig.value: {:?}", sig.value);
                    log::debug!("attesters_map: {:?}", attesters_map);

                    for (name, field_type) in &attesters_map {
                        let verify_result = match field_type {
                            FieldType::Certificate(cert) => match sig.cert_as_base64() {
                                Some(cert_base64) if &cert_base64 == cert => Cosign::verify_blob(
                                    cert_base64.trim(),
                                    &sig.value,
                                    &pae.clone().into_bytes(),
                                ),
                                _ => continue,
                            },
                            FieldType::PublicKey(public_key) => {
                                Cosign::verify_blob_with_public_key(
                                    public_key.trim(),
                                    &sig.value,
                                    &pae.clone().into_bytes(),
                                )
                            }
                            FieldType::SPKIKeyId(keyid)
                                if sig.keyid.is_some() && sig.keyid.as_ref().unwrap() == keyid =>
                            {
                                let envelope = input.as_json().to_string();
                                match public_key_for_keyid(envelope.as_str(), keyid).await {
                                    Ok(pubkey) => Cosign::verify_blob_with_public_key(
                                        &pubkey,
                                        &sig.value,
                                        &pae.clone().into_bytes(),
                                    ),
                                    Err(e) => {
                                        Err(SigstoreError::PublicKeyUnsupportedAlgorithmError(
                                            e.to_string(),
                                        ))
                                    }
                                }
                            }
                            _ => continue,
                        };

                        match verify_result {
                            Ok(_) => {
                                let mut attester_names: Vec<Arc<RuntimeValue>> = Vec::new();
                                let mut matched_subjects: Vec<Arc<RuntimeValue>> = Vec::new();
                                attester_names.push(Arc::new(RuntimeValue::from(name.to_string())));
                                log::debug!("Verification succeeded!");
                                let Ok(statement) =
                                    serde_json::from_str::<Statement>(&decoded_payload) else {
                                        return json_parse_error("payload");
                                    };

                                if statement._type != "https://in-toto.io/Statement/v0.1" {
                                    return invalid_type("_type", statement._type);
                                }

                                for subject in statement.subjects {
                                    for (alg, digest) in &subject.digest {
                                        if let Ok(hash) = hash(&blob, alg) {
                                            if &hash == digest {
                                                matched_subjects.push(Arc::new(
                                                    RuntimeValue::from(subject.name.to_string()),
                                                ));
                                            }
                                        }
                                    }
                                }
                                if !matched_subjects.is_empty() {
                                    let mut output = Object::new();
                                    output.set("predicate_type", statement.predicate_type);
                                    output.set("predicate", statement.predicate.clone());
                                    output.set("attester_names", attester_names.clone());
                                    output.set("matched_subjects", matched_subjects.clone());
                                    verified.push(Arc::new(RuntimeValue::Object(output)));
                                }
                            }
                            Err(e) => {
                                log::error!("verify_blob failed with {:?}", e);
                                return error(e.to_string());
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

async fn public_key_for_keyid(envelope: &str, keyid: &str) -> Result<String> {
    let bytes = envelope.trim().as_bytes();
    let hash = sha256(&bytes.to_vec());
    log::debug!("envelope hash: {:?}", hash);
    log::debug!("keyid: {:?}", keyid);
    let query = SearchIndex {
        email: None,
        public_key: None,
        hash: Some(hash),
    };
    let configuration = Configuration::default();
    let uuid_vec_res = index_api::search_index(&configuration, query).await;
    if let Ok(uuid_vec) = uuid_vec_res {
        log::debug!("Found uuids: {:?}", uuid_vec);
        for uuid in uuid_vec {
            let configuration = Configuration::default();
            let result = entries_api::get_log_entry_by_uuid(&configuration, &uuid).await;
            match result.unwrap().body {
                Body::intoto(value) => {
                    let pub_key_base64 = value.spec.get("publicKey").unwrap();
                    log::debug!("pub_key base64: {}", pub_key_base64);
                    let Ok(decoded_pub) = BASE64_STD_ENGINE.decode(pub_key_base64.as_str().unwrap()) else {
                        continue;
                    };
                    let Ok(decoded_pub_str) = std::str::from_utf8(&decoded_pub) else {
                        continue;
                    };
                    // We have base64 decoded the public key which we want to
                    // generate a fingerprint for so that we can compare to the
                    // keyid that was specified in the attesters field.
                    //
                    // The SignatureScheme is not important in our case as
                    // we are just using in-toto to parse the public key
                    // string. We will only use the KeyType (typ()) and the
                    // bytes (as_bytes()) functions below.
                    if let Ok(pub_key) =
                        PublicKey::from_pem_spki(decoded_pub_str, SignatureScheme::EcdsaP256Sha256)
                    {
                        match pub_key.typ() {
                            KeyType::Ecdsa => {
                                if let Ok(encoded_point) =
                                    EncodedPoint::<U32>::from_bytes(pub_key.as_bytes())
                                {
                                    let keydata =
                                        KeyData::Ecdsa(EcdsaPublicKey::NistP256(encoded_point));
                                    let fp = keydata.fingerprint(HashAlg::Sha256);
                                    log::info!("calculated fingerprint: {}", fp.to_string());
                                    log::info!("keyid      fingerprint: {}", keyid);
                                    if fp.to_string() == *keyid.to_string() {
                                        log::debug!(
                                            "Found public_key: {:?}",
                                            decoded_pub_str.replace("\n", "")
                                        );
                                        return Ok(decoded_pub_str.to_string());
                                    }
                                }
                            }
                            _ => {
                                return Err(anyhow::anyhow!(
                                    "Public key algorithm is currently not supported"
                                ))
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
    }
    Err(anyhow::anyhow!(
        "Could not find a public key for fingerprint"
    ))
}

fn sha256(bytes: &Vec<u8>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn hash(blob: &Vec<u8>, alg: &str) -> Result<String, anyhow::Error> {
    match alg {
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(blob);
            let hash = format!("{:x}", hasher.finalize());
            Ok(hash)
        }
        "sha512" => {
            let mut hasher = Sha512::new();
            hasher.update(blob);
            let hash = format!("{:x}", hasher.finalize());
            Ok(hash)
        }
        _ => Err(anyhow::anyhow!("Could not find a hasher for {alg}")),
    }
}

async fn get_blob<'v>(
    input: &Arc<RuntimeValue>,
    bindings: &Bindings,
    ctx: ExecutionContext<'v>,
    world: &'v World,
) -> Result<Vec<u8>, anyhow::Error> {
    if let Some(pattern) = bindings.get(BLOB) {
        let result = pattern
            .evaluate(input.clone(), ctx.push()?, bindings, world)
            .await?;

        if result.severity() < Severity::Error {
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

#[derive(Debug, Clone)]
enum FieldType {
    PublicKey(String),
    Certificate(String),
    SPKIKeyId(String),
    None,
}

fn get_attesters(param: &str, bindings: &Bindings) -> HashMap<String, FieldType> {
    let mut map = HashMap::new();
    if let Some(pattern) = bindings.get(param) {
        if let InnerPattern::List(list) = pattern.inner() {
            for item in list {
                if let InnerPattern::Object(p) = item.inner() {
                    let mut name: String = "".to_string();
                    for (_i, field) in p.fields().iter().enumerate() {
                        if let InnerPattern::Const(ValuePattern::String(value)) = field.ty().inner()
                        {
                            let val = value.to_string();
                            let field_type = match field.to_string().as_str() {
                                "name" => {
                                    name = val;
                                    continue;
                                }
                                "public_key" => FieldType::PublicKey(val),
                                "certificate" => FieldType::Certificate(val),
                                "spki_keyid" => FieldType::SPKIKeyId(val),
                                _ => FieldType::None,
                            };
                            map.insert(name.clone(), field_type);
                        };
                    }
                }
            }
        }
    }
    map
}

fn base64_decode_error(
    field: impl Into<Arc<str>>,
) -> Result<FunctionEvaluationResult, RuntimeError> {
    error(format!("Could not decode {} field to base64", field.into()))
}

fn json_parse_error(field: impl Into<Arc<str>>) -> Result<FunctionEvaluationResult, RuntimeError> {
    error(format!("Could not parse {}", field.into()))
}

fn missing_attesters() -> Result<FunctionEvaluationResult, RuntimeError> {
    error("At least one attester must be provided in the attesters parameter")
}

fn blob_error() -> Result<FunctionEvaluationResult, RuntimeError> {
    error("Blob could not be parsed. Please check if a data source directory was set.")
}

fn error(msg: impl Into<Arc<str>>) -> Result<FunctionEvaluationResult, RuntimeError> {
    Ok((Severity::Error, Rationale::InvalidArgument(msg.into())).into())
}

fn invalid_type(
    field: impl Into<Arc<str>>,
    value: impl Into<Arc<str>>,
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

    #[tokio::test]
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
              {name: "dan", certificate: "LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0tCk1JSUR3RENDQTBhZ0F3SUJBZ0lVTEpaajZlQVp0c1dkSUhGcktnK00rTFZkTkEwd0NnWUlLb1pJemowRUF3TXcKTnpFVk1CTUdBMVVFQ2hNTWMybG5jM1J2Y21VdVpHVjJNUjR3SEFZRFZRUURFeFZ6YVdkemRHOXlaUzFwYm5SbApjbTFsWkdsaGRHVXdIaGNOTWpNd016RTBNVEF5TlRBMVdoY05Nak13TXpFME1UQXpOVEExV2pBQU1Ga3dFd1lICktvWkl6ajBDQVFZSUtvWkl6ajBEQVFjRFFnQUVtSUF2WFZMVGg2NkUzV2RXUkZac1ZTSE9VQ2swbUwrazRLSXYKYU4zOWhHekhncHozalp2Ylp3NnhTaHJidVZYVW4wMUFQck0vUWh0YVZhMWJtZUJLV0tPQ0FtVXdnZ0poTUE0RwpBMVVkRHdFQi93UUVBd0lIZ0RBVEJnTlZIU1VFRERBS0JnZ3JCZ0VGQlFjREF6QWRCZ05WSFE0RUZnUVVkbkhyCjlKdFFlQlFHVnhtU0JkWHFBMnhDVXlVd0h3WURWUjBqQkJnd0ZvQVUzOVBwejFZa0VaYjVxTmpwS0ZXaXhpNFkKWkQ4d2ZRWURWUjBSQVFIL0JITXdjWVp2YUhSMGNITTZMeTluYVhSb2RXSXVZMjl0TDNOc2MyRXRabkpoYldWMwpiM0pyTDNOc2MyRXRaMmwwYUhWaUxXZGxibVZ5WVhSdmNpOHVaMmwwYUhWaUwzZHZjbXRtYkc5M2N5OWlkV2xzClpHVnlYMmR2WDNOc2MyRXpMbmx0YkVCeVpXWnpMM1JoWjNNdmRqRXVOUzR3TURrR0Npc0dBUVFCZzc4d0FRRUUKSzJoMGRIQnpPaTh2ZEc5clpXNHVZV04wYVc5dWN5NW5hWFJvZFdKMWMyVnlZMjl1ZEdWdWRDNWpiMjB3RWdZSwpLd1lCQkFHRHZ6QUJBZ1FFY0hWemFEQTJCZ29yQmdFRUFZTy9NQUVEQkNoaU5qQXhZek13WWpNeFl6UmxPRE14CllqRmhPRFF4T0daa01Ua3paakEwWXpJM05XUXlNVEJqTUJNR0Npc0dBUVFCZzc4d0FRUUVCVWR2SUVOSk1ERUcKQ2lzR0FRUUJnNzh3QVFVRUkzTmxaV1IzYVc1bkxXbHZMM05sWldSM2FXNW5MV2R2YkdGdVp5MWxlR0Z0Y0d4bApNQjhHQ2lzR0FRUUJnNzh3QVFZRUVYSmxabk12ZEdGbmN5OTJNQzR4TGpFMU1JR0tCZ29yQmdFRUFkWjVBZ1FDCkJId0VlZ0I0QUhZQTNUMHdhc2JIRVRKakdSNGNtV2MzQXFKS1hyamVQSzMvaDRweWdDOHA3bzRBQUFHRzM2YnkKSmdBQUJBTUFSekJGQWlFQTlyYnVNRDNoeHFkbTRCU1kxNmNncGlFMCtabWZITk9FbjhrblJqenB3WkVDSURnaAo2a1g0d005ZDVJUGlsdkZ6bjJ4KytJU0tYaU9LdmZyS24xa0tUaFR3TUFvR0NDcUdTTTQ5QkFNREEyZ0FNR1VDCk1FTy9qeG11aVBpUGRmVkREY1hBRVowSFRSVXA5V3Bjc2Y4dlhkdTFqODRVd291ZzUzaXZsdW1Yb0ZxN2hlSzEKdGdJeEFQQ29sOTk3QTgrTnFLVWllcmw5RGFFd2hBcG5HWlVTNXJ2MS9TcWpwbEpJSGhFTHFUMzZoNjR5dzl1QwprUDhlRGc9PQotLS0tLUVORCBDRVJUSUZJQ0FURS0tLS0tCg=="}
            ]

            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            RuntimeValue::from(&input_json)
        ).await;
        assert_satisfied!(&result);

        let output = result.output().as_json();
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
        assert_eq!(output.get("attester_names").unwrap()[0], "dan");
        assert_eq!(
            output.get("matched_subjects").unwrap()[0],
            "binary-linux-amd64"
        );
    }

    #[tokio::test]
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
              {name: "dan", certificate: "dummy-value"}
            ]

            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            RuntimeValue::from(&input_json),
        )
        .await;
        assert_not_satisfied!(result);
    }

    #[tokio::test]
    async fn verify_envelope_invalid_payload_type() {
        let input = json!({
           "payloadType": "application/vnd.in-typo+json",
           "payload": "dummy",
           "signatures": [{"sig": "dummy", "cert": "anything"}]
        });
        let value = RuntimeValue::from(input);
        let result = test_patterns(
            r#"
            pattern blob = *data::from<"intoto/binary-linux-amd64">
            pattern attesters = [{name: "dan", certificate: "bogus"}]
            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            value,
        )
        .await;
        assert_not_satisfied!(&result);
        assert_eq!(
            result.rationale().reason(),
            "invalid argument: invalid payloadType specified application/vnd.in-typo+json"
        );
    }

    #[actix_rt::test]
    async fn verify_envelope_empty_attesters() {
        let input_str = fs::read_to_string(
            test_data_dir()
                .join("intoto")
                .join("tekton-chains-envelope.json"),
        )
        .unwrap();
        let input_json: serde_json::Value = serde_json::from_str(&input_str).unwrap();
        let result = test_patterns(
            r#"
            pattern blob = *data::from<"intoto/tekton-example.blob">

            pattern attesters = []

            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            RuntimeValue::from(&input_json),
        )
        .await;
        assert_not_satisfied!(&result);
        assert_eq!(
            result.rationale().reason(),
            "invalid argument: At least one attester must be provided in the attesters parameter",
        );
    }

    #[actix_rt::test]
    async fn verify_envelope_using_public_key() {
        let input_str = fs::read_to_string(
            test_data_dir()
                .join("intoto")
                .join("tekton-chains-envelope.json"),
        )
        .unwrap();
        let input_json: serde_json::Value = serde_json::from_str(&input_str).unwrap();
        let result = test_patterns(
            r#"
            pattern blob = *data::from<"intoto/tekton-example.blob">

            pattern attesters = [
              {name: "dan", public_key: "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEqiLuArRcZCY1s650rgKUDpj7f+b8
9HMu3K/PDaUcR9kcyyXY8q6U+TFTkc9u84wJTsZe21wBPd/STPEzo0JrzQ==
-----END PUBLIC KEY-----"}
            ]

            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            RuntimeValue::from(&input_json),
        )
        .await;
        assert_satisfied!(&result);

        let output = result.output().as_json();
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
        assert_eq!(output.get("attester_names").unwrap()[0], "dan");
    }

    #[actix_rt::test]
    async fn verify_envelope_using_keyid() {
        let input_str = fs::read_to_string(
            test_data_dir()
                .join("intoto")
                .join("tekton-chains-envelope.json"),
        )
        .unwrap();
        let input_json: serde_json::Value = serde_json::from_str(&input_str).unwrap();
        let result = test_patterns(
            r#"
            pattern blob = *data::from<"intoto/tekton-example.blob">

            pattern attesters = [
              {name: "dan", spki_keyid: "SHA256:caEJWYJSxy1SVF2KObm5Rr3Yt6xIb4T2w56FHtCg8WI"}
            ]

            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            RuntimeValue::from(&input_json),
        )
        .await;
        assert_satisfied!(&result);

        let output = result.output().as_json();
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
        assert_eq!(output.get("attester_names").unwrap()[0], "dan");
    }

    #[actix_rt::test]
    async fn verify_envelope_using_keyid_wrong_blob() {
        let input_str = fs::read_to_string(
            test_data_dir()
                .join("intoto")
                .join("tekton-chains-envelope.json"),
        )
        .unwrap();
        let input_json: serde_json::Value = serde_json::from_str(&input_str).unwrap();
        let result = test_patterns(
            r#"
            pattern blob = *data::from<"intoto/tekton-example-invalid.blob">

            pattern attesters = [
              {name: "dan", spki_keyid: "SHA256:caEJWYJSxy1SVF2KObm5Rr3Yt6xIb4T2w56FHtCg8WI"}
            ]

            pattern test-pattern = intoto::verify-envelope<attesters, blob>"#,
            RuntimeValue::from(&input_json),
        )
        .await;
        assert_not_satisfied!(result);
    }

    fn payload_as_json(input: &serde_json::Value) -> serde_json::Value {
        let envelope: Envelope = serde_json::from_value(input.clone()).unwrap();
        let payload_base64 = envelope.payload_from_base64().unwrap();
        let payload: serde_json::Value = serde_json::from_str(&payload_base64).unwrap();
        payload
    }
}
