use crate::ui::rationale::Rationalizer;
use actix_web::{
    http::header::ContentType,
    web::{BytesMut, Payload},
};
use futures_util::StreamExt;
use seedwing_policy_engine::runtime::{EvaluationResult, Response};
use serde::Deserialize;
use serde_json::Error;

pub async fn parse(body: &mut Payload) -> Result<serde_json::Value, Error> {
    let mut content = BytesMut::new();
    while let Some(Ok(bit)) = body.next().await {
        content.extend_from_slice(&bit);
    }
    serde_json::from_slice(&content)
        .or_else(|_| serde_yaml::from_slice::<serde_json::Value>(&content))
        .map_err(serde::de::Error::custom)
}

#[derive(Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Html,
    Json,
    Yaml,
}

impl Format {
    pub fn format(&self, result: &EvaluationResult, collapse: bool) -> String {
        let response = if let Self::Html = self {
            Response::default()
        } else if collapse {
            Response::new(result).collapse()
        } else {
            Response::new(result)
        };
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
