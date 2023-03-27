use seedwing_policy_engine::{
    lang::builder::Builder,
    runtime::{response::Name, sources::Ephemeral, EvalContext, EvaluationResult, Response, World},
};
use serde_json::Value;
use std::fmt::{Debug, Display};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Reason {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
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
            name: match value.name {
                Name::Field(field) => format!("field:{field}"),
                Name::Pattern(Some(pattern)) => pattern.to_string(),
                Name::Pattern(None) => String::new(),
            },
            failed: match value.satisfied {
                true => None,
                false => Some(value.reason),
            },
            rationale: value
                .rationale
                .into_iter()
                .map(|s| s.into())
                .collect::<Vec<_>>(),
        }
    }
}

/// Build a world with the provided source, or panic.
///
/// The package of the source is `test`.
pub async fn build(source: impl Into<String>) -> World {
    let source = Ephemeral::new("test", source);
    let mut builder = Builder::new();
    builder.build(source.iter()).unwrap();
    builder.finish().await.unwrap()
}

pub async fn assert_eval<E>(source: impl Into<String>, input: Value, expected: E)
where
    E: TryInto<Reason>,
    E::Error: Debug,
{
    let result = eval_test(source, input).await;

    let response = Response::new(&result);
    let reasons = Reason::from(response);

    let expected: Reason = expected.try_into().unwrap();

    assert_eq!(reasons, expected);
}

/// Evaluate a pattern named "test"
pub async fn eval_test(source: impl Into<String>, input: Value) -> EvaluationResult {
    eval(source, "test", input).await
}

pub async fn eval(
    source: impl Into<String>,
    pattern: impl Display,
    input: Value,
) -> EvaluationResult {
    let name = format!("test::{pattern}");
    let world = build(source).await;

    world
        .evaluate(name, input, EvalContext::default())
        .await
        .unwrap()
}
