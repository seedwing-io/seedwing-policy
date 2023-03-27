use seedwing_policy_engine::runtime::Response;
use seedwing_policy_engine::{
    lang::builder::Builder,
    runtime::{sources::Ephemeral, EvalContext, EvaluationResult, World},
};
use serde_json::{json, Value};
use std::fmt::Display;

#[tokio::test]
async fn simple() -> anyhow::Result<()> {
    let result = eval(
        r#"
#[explain("Not foo")]
pattern foo = {}
"#,
        "foo",
        json!(false),
    )
    .await;

    assert_eq!(result.satisfied(), false);
    let response = Response::new(&result);

    assert_eq!(response.reason, "Not foo");

    Ok(())
}

/// Build a world with the provided source, or panic.
///
/// The package of the source is `test`.
async fn build(source: impl Into<String>) -> World {
    let source = Ephemeral::new("test", source);
    let mut builder = Builder::new();
    builder.build(source.iter()).unwrap();
    builder.finish().await.unwrap()
}

async fn eval(source: impl Into<String>, pattern: impl Display, input: Value) -> EvaluationResult {
    let world = build(source).await;
    world
        .evaluate(format!("test::{pattern}"), input, EvalContext::default())
        .await
        .unwrap()
}
