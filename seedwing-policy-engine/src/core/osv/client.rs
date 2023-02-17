use chrono::{DateTime, Utc};
use http::StatusCode;
use serde::{Deserialize, Serialize};

const OSV_URL: &str = "https://api.osv.dev/v1/query";
const OSV_VULN_URL: &str = "https://api.osv.dev/v1/vulns";
const OSV_BATCH_URL: &str = "https://api.osv.dev/v1/querybatch";

pub struct OsvClient {
    client: reqwest::Client,
}

impl OsvClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
    pub async fn query_batch(
        &self,
        queries: &Vec<OsvQuery>,
    ) -> Result<OsvBatchResponse, anyhow::Error> {
        let query = OsvBatchQuery { queries };
        let response = self.client.post(OSV_BATCH_URL).json(&query).send().await;
        match response {
            Ok(r) if r.status() == StatusCode::OK => {
                r.json::<OsvBatchResponse>().await.map_err(|e| e.into())
            }
            Ok(r) => Err(anyhow::anyhow!("Error querying package: {:?}", r)),
            Err(e) => {
                log::warn!("Error querying OSV: {:?}", e);
                Err(e.into())
            }
        }
    }

    pub async fn query(&self, payload: OsvQuery) -> Result<OsvResponse, anyhow::Error> {
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

    pub async fn fetch_id(&self, id: &str) -> Result<OsvVulnerability, anyhow::Error> {
        let response = self
            .client
            .get(format!("{}/{id}", OSV_VULN_URL))
            .send()
            .await;
        match response {
            Ok(r) if r.status() == StatusCode::OK => {
                r.json::<OsvVulnerability>().await.map_err(|e| e.into())
            }
            Ok(r) => Err(anyhow::anyhow!("Error fetch vulnerability {}: {:?}", id, r)),
            Err(e) => {
                log::warn!("Error fetching vulnerability {}: {:?}", id, e);
                Err(e.into())
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OsvBatchQuery<'a> {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    queries: &'a Vec<OsvQuery>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvQuery {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    version: Option<String>,
    package: OsvPackageQuery,
}

impl From<&str> for OsvQuery {
    fn from(purl: &str) -> OsvQuery {
        Self {
            version: None,
            package: OsvPackageQuery {
                name: None,
                ecosystem: None,
                purl: Some(purl.to_string()),
            },
        }
    }
}

impl From<(&str, &str, &str)> for OsvQuery {
    fn from(values: (&str, &str, &str)) -> OsvQuery {
        let ecosystem = values.0;
        let name = values.1;
        let version = values.2;
        OsvQuery {
            version: Some(version.to_string()),
            package: OsvPackageQuery {
                name: Some(name.to_string()),
                ecosystem: Some(ecosystem.to_string()),
                purl: None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvPackageQuery {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ecosystem: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    purl: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvBatchResponse {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub results: Vec<OsvResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvResponse {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub vulns: Vec<OsvVulnerability>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsvVulnerability {
    #[serde(alias = "schemaVersion")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub schema_version: Option<String>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub published: Option<DateTime<Utc>>,
    pub modified: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub withdrawn: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
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
