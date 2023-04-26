use crate::ui::rationale::Rationalizer;
use seedwing_policy_engine::{
    lang::Severity,
    runtime::{response::ResponseFields, EvaluationResult, Response},
};
use serde::Deserialize;
use serde_view::View;
use std::collections::HashSet;
use std::fmt::{self, Display};

#[derive(Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Html,
    Json,
    Yaml,
}

pub enum FormatError {
    Json(serde_json::Error),
    Yaml(serde_yaml::Error),
    InvalidViewField,
}

impl Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Json(e) => e.fmt(f),
            Self::Yaml(e) => e.fmt(f),
            Self::InvalidViewField => write!(f, "Invalid view field"),
        }
    }
}

impl Format {
    pub fn format(
        &self,
        result: &EvaluationResult,
        collapse: bool,
        fields: Option<String>,
    ) -> Result<String, FormatError> {
        let mut response = Response::new(result);
        if collapse {
            response = response.collapse(Severity::Error);
        }

        let formatter = match self {
            // FIXME: Rationalizer should use `response` too, currently it ignored the collapse flag
            Self::Html => return Ok(Rationalizer::new(result).rationale()),
            Self::Json => |response| serde_json::to_string(&response).map_err(FormatError::Json),
            Self::Yaml => |response| serde_yaml::to_string(&response).map_err(FormatError::Yaml),
        };

        let fields = fields
            .map(|fields| {
                fields
                    .split(",")
                    .map(std::str::FromStr::from_str)
                    .collect::<Result<HashSet<ResponseFields>, _>>()
            })
            .transpose()
            .map_err(|()| FormatError::InvalidViewField)?
            .unwrap_or_default();

        formatter(&response.as_view().with_fields(fields))
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Html => "text/html; charset=utf-8",
            Self::Json => "application/json",
            Self::Yaml => "application/yaml",
        }
    }
}

impl From<String> for Format {
    fn from(name: String) -> Self {
        match name.as_str() {
            "json" | "application/json" => Self::Json,
            "yaml" | "application/yaml" | "application/x-yaml" | "text/x-yaml" => Self::Yaml,
            _ => Self::Html,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Format;
    use seedwing_policy_engine::{
        lang::builder::Builder,
        runtime::{sources::Ephemeral, EvalContext},
    };
    use serde_json::json;

    #[tokio::test]
    async fn unknown_field() {
        let src = Ephemeral::new("test", r#"pattern fubar = lang::or<["foo", "bar"]>"#);
        let mut builder = Builder::new();
        let _ = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::fubar", json!("foo"), EvalContext::default())
            .await
            .unwrap();
        assert!(Format::Json
            .format(&result, true, Some(String::from("name")))
            .is_ok());
        assert!(Format::Json
            .format(&result, true, Some(String::from("fart")))
            .is_err());
    }
}
