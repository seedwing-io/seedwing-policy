use actix_web::{
    http::header::ContentType,
    web::{BytesMut, Payload},
};
use futures_util::StreamExt;
use seedwing_policy_engine::runtime::{rationale::Rationale, EvaluationResult, TypeName};
use serde::{Deserialize, Serialize};
use serde_json::Error;

use super::rationale::Rationalizer;

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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<TypeName>,
    input: serde_json::Value,
    satisfied: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    reason: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rationale: Vec<Response>,
}

impl Response {
    pub fn new(result: &EvaluationResult) -> Self {
        Self {
            name: result.ty().name(),
            input: result.input().as_json(),
            satisfied: result.satisfied(),
            reason: reason(result.rationale()),
            rationale: support(result),
        }
    }
    fn collapse(mut self) -> Self {
        self.rationale = if self.satisfied {
            Vec::new()
        } else {
            unsatisfied_leaves(self.rationale)
        };
        self
    }
}

fn unsatisfied_leaves(tree: Vec<Response>) -> Vec<Response> {
    let mut result = Vec::new();
    for i in tree.into_iter() {
        if i.satisfied {
            continue;
        }
        if i.rationale.is_empty() {
            result.push(i);
        } else {
            result.append(&mut unsatisfied_leaves(i.rationale.clone()));
        }
    }
    result
}

fn reason(rationale: &Rationale) -> String {
    match rationale {
        Rationale::Anything => "anything is satisfied by anything".to_string(),
        Rationale::Nothing
        | Rationale::Const(_)
        | Rationale::Primordial(_)
        | Rationale::Expression(_) => "".to_string(),
        Rationale::Object(_) => if rationale.satisfied() {
            "because all fields were satisfied"
        } else {
            "because not all fields were satisfied"
        }
        .to_string(),
        Rationale::List(_terms) => if rationale.satisfied() {
            "because all members were satisfied"
        } else {
            "because not all members were satisfied"
        }
        .to_string(),
        Rationale::Chain(_terms) => if rationale.satisfied() {
            "because the chain was satisfied"
        } else {
            "because the chain was not satisfied"
        }
        .to_string(),
        Rationale::NotAnObject => "not an object".to_string(),
        Rationale::NotAList => "not a list".to_string(),
        Rationale::MissingField(name) => format!("missing field: {name}"),
        Rationale::InvalidArgument(name) => format!("invalid argument: {name}"),
        Rationale::Function(_, _, _) => String::new(),
        Rationale::Refinement(_primary, _refinement) => todo!(),
    }
}

fn support(result: &EvaluationResult) -> Vec<Response> {
    match result.rationale() {
        Rationale::Object(fields) => fields
            .iter()
            .map(|(_, r)| r.as_ref().map(|r| Response::new(r)))
            .flatten()
            .collect(),
        Rationale::List(terms) | Rationale::Chain(terms) | Rationale::Function(_, _, terms) => {
            terms.iter().map(Response::new).collect()
        }
        Rationale::Anything
        | Rationale::Nothing
        | Rationale::NotAnObject
        | Rationale::NotAList
        | Rationale::MissingField(_)
        | Rationale::InvalidArgument(_)
        | Rationale::Const(_)
        | Rationale::Primordial(_)
        | Rationale::Refinement(_, _)
        | Rationale::Expression(_) => Vec::new(),
    }
}

#[cfg(test)]
mod test {
    use seedwing_policy_engine::lang::builder::Builder;
    use seedwing_policy_engine::lang::lir::EvalContext;
    use seedwing_policy_engine::runtime::sources::Ephemeral;
    use serde_json::json;

    #[tokio::test]
    async fn happy_any_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern foo = list::any<42>
        "#,
        );
        let mut builder = Builder::new();
        let _ = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::foo", json!([1, 42, 99]), EvalContext::default())
            .await
            .unwrap();
        assert!(result.satisfied());
        assert_eq!(
            r#"{"name":"list::any","input":[1,42,99],"satisfied":true,"rationale":[{"input":1,"satisfied":false},{"input":42,"satisfied":true},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&super::Response::new(&result)).unwrap()
        );
        assert_eq!(
            r#"{"name":"list::any","input":[1,42,99],"satisfied":true}"#,
            serde_json::to_string(&super::Response::new(&result).collapse()).unwrap()
        );
    }
}
