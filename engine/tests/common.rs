// Some tests don't use all functions, that might trigger warnings
#![allow(unused)]

use seedwing_policy_engine::{
    lang::{builder::Builder, Severity},
    runtime::{
        is_default, response::Name, sources::Ephemeral, EvalContext, EvaluationResult, Response,
        World,
    },
    test::Reason,
};
use serde_json::Value;
use std::fmt::{Debug, Display};

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
