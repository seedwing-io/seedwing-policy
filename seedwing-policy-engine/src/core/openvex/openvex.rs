//! Types for describing an OpenVEX document.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The VEX type represents a VEX document and all of its contained information.
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenVex {
    #[serde(flatten)]
    pub metadata: Metadata,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub statements: Vec<Statement>,
}

/// The Metadata type represents the metadata associated with a VEX document.
#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    /// Context is the URL pointing to the jsonld context definition
    #[serde(rename = "@context")]
    pub context: String,

    // ID is the identifying string for the VEX document. This should be unique per document.
    #[serde(rename = "@id")]
    pub id: String,

    /// Author is the identifier for the author of the VEX statement, ideally a common
    /// name, may be a URI. [author] is an individual or organization. [author]
    /// identity SHOULD be cryptographically associated with the signature of the VEX
    /// statement or document or transport.
    pub author: String,

    /// AuthorRole describes the role of the document Author.
    pub role: String,

    /// Timestamp defines the time at which the document was issued.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timestamp: Option<DateTime<Utc>>,

    /// Version is the document version. It must be incremented when any content
    /// within the VEX document changes, including any VEX statements included within
    /// the VEX document.
    pub version: String,

    /// Tooling expresses how the VEX document and contained VEX statements were
    /// generated. It's optional. It may specify tools or automated processes used in
    /// the document or statement generation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tooling: Option<String>,

    /// Supplier is an optional field.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub supplier: Option<String>,
}

/// A Statement is a declaration conveying a single [status] for a single
/// [vul_id] for one or more [product_id]s. A VEX Statement exists within a VEX
/// Document.
#[derive(Debug, Serialize, Deserialize)]
pub struct Statement {
    /// [vul_id] SHOULD use existing and well known identifiers, for example:
    /// CVE, the Global Security Database (GSD), or a supplier’s vulnerability
    /// tracking system. It is expected that vulnerability identification systems
    /// are external to and maintained separately from VEX.
    ///
    /// [vul_id] MAY be URIs or URLs.
    /// [vul_id] MAY be arbitrary and MAY be created by the VEX statement [author].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub vulnerability: Option<String>,

    /// Textual description of a vulnerability.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub vuln_description: Option<String>,

    /// Timestamp is the time at which the information expressed in the Statement
    /// was known to be true.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timestamp: Option<DateTime<Utc>>,

    /// ProductIdentifiers
    /// Product details MUST specify what Status applies to.
    /// Product details MUST include [product_id] and MAY include [subcomponent_id].
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub products: Vec<String>,

    /// SubComponents
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub subcomponents: Vec<String>,

    /// A VEX statement MUST provide Status of the vulnerabilities with respect to the
    /// products and components listed in the statement. Status MUST be one of the
    /// Status const values, some of which have further options and requirements.
    pub status: Status,

    /// [status_notes] MAY convey information about how [status] was determined
    /// and MAY reference other VEX information.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status_notes: Option<String>,

    /// For ”not_affected” status, a VEX statement MUST include a status Justification
    /// that further explains the status.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub justification: Option<Justification>,

    /// For ”not_affected” status, a VEX statement MAY include an ImpactStatement
    /// that contains a description why the vulnerability cannot be exploited.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub impact_statement: Option<String>,

    /// For "affected" status, a VEX statement MUST include an ActionStatement that
    /// SHOULD describe actions to remediate or mitigate [vul_id].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub action_statement: Option<String>,

    /// Action statement timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub action_statement_timestamp: Option<DateTime<Utc>>,
}

/// Status describes the exploitability status of a component with respect to a
/// vulnerability.
#[derive(Debug, Serialize, Deserialize)]
pub enum Status {
    /// NotAffected means no remediation or mitigation is required.
    #[serde(rename = "not_affected")]
    NotAffected,

    /// Affected means actions are recommended to remediate or mitigate.
    #[serde(rename = "affected")]
    Affected,

    /// Fixed means the listed products or components have been remediated (by including fixes).
    #[serde(rename = "fixed")]
    Fixed,

    /// UnderInvestigation means the author of the VEX statement is investigating.
    #[serde(rename = "under_investigation")]
    UnderInvestigation,
}

/// Justification describes why a given component is not affected by a
/// vulnerability.
#[derive(Debug, Serialize, Deserialize)]
pub enum Justification {
    /// ComponentNotPresent means the vulnerable component is not included in the artifact.
    ///
    /// ComponentNotPresent is a strong justification that the artifact is not affected.
    #[serde(rename = "component_not_present")]
    ComponentNotPresent,

    /// VulnerableCodeNotPresent means the vulnerable component is included in
    /// artifact, but the vulnerable code is not present. Typically, this case occurs
    /// when source code is configured or built in a way that excluded the vulnerable
    /// code.
    ///
    /// VulnerableCodeNotPresent is a strong justification that the artifact is not affected.
    #[serde(rename = "vulnerable_code_not_present")]
    VulnerableCodeNotPresent,

    /// VulnerableCodeNotInExecutePath means the vulnerable code (likely in
    /// [subcomponent_id]) can not be executed as it is used by [product_id].
    /// Typically, this case occurs when [product_id] includes the vulnerable
    /// [subcomponent_id] and the vulnerable code but does not call or use the
    /// vulnerable code.
    #[serde(rename = "vulnerable_code_not_in_execute_path")]
    VulnerableCodeNotInExecutePath,

    /// VulnerableCodeCannotBeControlledByAdversary means the vulnerable code cannot
    /// be controlled by an attacker to exploit the vulnerability.
    ///
    /// This justification could be difficult to prove conclusively.
    #[serde(rename = "vulnerable_code_cannot_be_controlled_by_adversary")]
    VulnerableCodeCannotBeControlledByAdversary,

    /// InlineMitigationsAlreadyExist means [product_id] includes built-in protections
    /// or features that prevent exploitation of the vulnerability. These built-in
    /// protections cannot be subverted by the attacker and cannot be configured or
    /// disabled by the user. These mitigations completely prevent exploitation based
    /// on known attack vectors.
    ///
    /// This justification could be difficult to prove conclusively. History is
    /// littered with examples of mitigation bypasses, typically involving minor
    /// modifications of existing exploit code.
    #[serde(rename = "inline_mitigations_already_exist")]
    InlineMitigationsAlreadyExist,
}
