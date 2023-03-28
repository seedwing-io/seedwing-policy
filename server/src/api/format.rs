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

impl Format {
    pub fn format(
        &self,
        result: &EvaluationResult,
        collapse: bool,
        fields: Option<String>,
    ) -> String {
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
            Self::Html => Rationalizer::new(result).rationale(),
            Self::Json => serde_json::to_string_pretty(&response).unwrap(),
            Self::Yaml => serde_yaml::to_string(&response).unwrap(),
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
