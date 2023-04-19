use crate::ui::rationale::Rationalizer;
use seedwing_policy_engine::{
    lang::Severity,
    runtime::{EvaluationResult, Response},
};
use serde::Deserialize;
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
}

impl Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Json(e) => e.fmt(f),
            Self::Yaml(e) => e.fmt(f),
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
        let response = if let Self::Html = self {
            // in case it's HTML, this value will be ignored later on
            Response::default()
        } else if collapse {
            Response::new(result).collapse(Severity::Error)
        } else {
            Response::new(result)
        };
        if let Some(s) = fields {
            response.filter(&s).map_err(FormatError::Json)?;
        }
        match self {
            // FIXME: Rationalizer should use `response` too, currently it ignored the collapse flag
            Self::Html => Ok(Rationalizer::new(result).rationale()),
            Self::Json => serde_json::to_string_pretty(&response).map_err(FormatError::Json),
            Self::Yaml => serde_yaml::to_string(&response).map_err(FormatError::Yaml),
        }
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
