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
    pub vulns: Vec<OsvVulnerability>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvVulnerability {
    #[serde(alias = "schemaVersion")]
    pub schema_version: String,
    pub id: String,
    pub published: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub withdrawn: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related: Vec<String>,
    pub summary: String,
    pub details: String,
    pub affected: Vec<OsvAffected>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub references: Vec<OsvReference>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub severity: Vec<OsvSeverity>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub credits: Vec<OsvCredit>,

    #[serde(alias = "databaseSpecific")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database_specific: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvAffected {
    pub package: OsvPackage,
    pub ranges: Vec<OsvRange>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub versions: Vec<String>,

    #[serde(alias = "ecosystemSpecific")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ecosystem_specific: Option<serde_json::Value>,

    #[serde(alias = "databaseSpecific")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database_specific: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvPackage {
    pub name: String,
    pub ecosystem: String,
    pub purl: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvRange {
    pub r#type: OsvRangeType,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub repo: Option<String>,
    pub events: Vec<OsvEvent>,
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
    pub introduced: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fixed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvReference {
    pub r#type: OsvReferenceType,
    pub url: String,
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
    pub r#type: OsvSeverityType,
    pub score: String,
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
    pub name: String,
    pub contact: Vec<String>,
}
