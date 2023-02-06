use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::{Deserialize, Serialize};

const OSV_URL: &str = "https://api.osv.dev/v1/query";
pub struct OsvClient {
    client: reqwest::Client,
}

impl OsvClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
    pub async fn query(
        &self,
        ecosystem: &str,
        name: &str,
        version: &str,
    ) -> Result<OsvResponse, anyhow::Error> {
        let payload = OsvQuery {
            version: version.to_string(),
            package: OsvPackageQuery {
                name: name.to_string(),
                ecosystem: ecosystem.to_string(),
            },
        };
        log::info!("Querying for {:?}", payload);
        let response = self.client.post(OSV_URL).json(&payload).send().await;
        match response {
            Ok(r) if r.status() == StatusCode::OK => {
                r.json::<OsvResponse>().await.map_err(|e| e.into())
            }
            Ok(r) => Err(anyhow::anyhow!("Error querying package: {:?}", r)),
            Err(e) => {
                log::warn!("Error querying OSV: {:?}", e);
                Err(e.into())
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvQuery {
    version: String,
    package: OsvPackageQuery,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvPackageQuery {
    name: String,
    ecosystem: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvResponse {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    vulns: Vec<OsvVulnerability>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvVulnerability {
    #[serde(alias = "schemaVersion")]
    schema_version: String,
    id: String,
    published: DateTime<Utc>,
    modified: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    withdrawn: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    aliases: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    related: Vec<String>,
    summary: String,
    details: String,
    affected: Vec<OsvAffected>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    references: Vec<OsvReference>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    severity: Vec<OsvSeverity>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    credits: Vec<OsvCredit>,

    #[serde(alias = "databaseSpecific")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    database_specific: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvAffected {
    package: OsvPackage,
    ranges: Vec<OsvRange>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    versions: Vec<String>,

    #[serde(alias = "ecosystemSpecific")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ecosystem_specific: Option<serde_json::Value>,

    #[serde(alias = "databaseSpecific")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    database_specific: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvPackage {
    name: String,
    ecosystem: String,
    purl: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvRange {
    r#type: OsvRangeType,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    repo: Option<String>,
    events: Vec<OsvEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OsvRangeType {
    #[serde(rename = "UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "GIT")]
    Git,
    #[serde(rename = "SEMVER")]
    SemVer,
    #[serde(rename = "ECOSYSTEM")]
    Ecosystem,
}

impl Default for OsvRangeType {
    fn default() -> Self {
        Self::Unspecified
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvEvent {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    introduced: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    fixed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    limit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvReference {
    r#type: OsvReferenceType,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OsvReferenceType {
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "WEB")]
    Web,
    #[serde(rename = "ADVISORY")]
    Advisory,
    #[serde(rename = "REPORT")]
    Report,
    #[serde(rename = "FIX")]
    Fix,
    #[serde(rename = "PACKAGE")]
    Package,
    #[serde(rename = "ARTICLE")]
    Article,
}

impl Default for OsvReferenceType {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvSeverity {
    r#type: OsvSeverityType,
    score: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OsvSeverityType {
    #[serde(rename = "UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "CVSS_V3")]
    CvssV3,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvCredit {
    name: String,
    contact: Vec<String>,
}
