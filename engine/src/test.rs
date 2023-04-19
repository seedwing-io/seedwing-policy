use crate::{
    lang::Severity,
    runtime::{is_default, response::Name, Response},
};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Reason {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rationale: Vec<Reason>,
}

impl TryFrom<Value> for Reason {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl From<Response> for Reason {
    fn from(value: Response) -> Self {
        Self {
            name: match value.name() {
                Name::Field(field) => format!("field:{field}"),
                Name::Pattern(Some(pattern)) => pattern.to_string(),
                Name::Pattern(None) => String::new(),
            },
            severity: value.severity(),
            reason: value.reason(),
            rationale: value
                .rationale()
                .into_iter()
                .map(|s| s.into())
                .collect::<Vec<_>>(),
        }
    }
}
