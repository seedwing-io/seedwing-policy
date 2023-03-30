use std::fmt::{self, Display};

use crate::ui::rationale::Rationalizer;
use actix_web::http::header::ContentType;
use seedwing_policy_engine::runtime::{EvaluationResult, Response};
use serde::Deserialize;

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
        let mut response = if let Self::Html = self {
            Response::default()
        } else if collapse {
            Response::new(result).collapse()
        } else {
            Response::new(result)
        };
        if let Some(s) = fields {
            response.filter(&s);
        }
        match self {
            Self::Html => Ok(Rationalizer::new(result).rationale()),
            Self::Json => serde_json::to_string_pretty(&response).map_err(|e| FormatError::Json(e)),
            Self::Yaml => serde_yaml::to_string(&response).map_err(|e| FormatError::Yaml(e)),
        }
    }
    pub fn content_type(&self) -> ContentType {
        match self {
            Self::Html => ContentType::html(),
            Self::Json => ContentType::json(),
            Self::Yaml => ContentType::plaintext(), // TODO: not this?
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
