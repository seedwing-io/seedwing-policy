use serde_json::json;

mod common;

use common::*;

#[tokio::test]
async fn simple_pattern() -> anyhow::Result<()> {
    assert_eval(
        r#"
#[reason("Not foo")]
pattern test = {}
"#,
        json!(false),
        json!({
            "name": "test::test",
            "severity": "error",
            "reason": "Not foo",
        }),
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn simple_field() -> anyhow::Result<()> {
    assert_eval(
        r#"
pattern test = {
  #[reason("Not baz")]
  bar: "baz"
}
"#,
        json!({"bar": "bar"}),
        json!({
            "name": "test::test",
            "severity": "error",
            "reason": "Because not all fields were satisfied",
            "rationale": [
                {
                    "name": "field:bar",
                    "severity": "error",
                    "reason": "Not baz",
                    "rationale": [{
                        "severity": "error",
                        "reason": "Not baz"
                    }]
                }
            ]
        }),
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn fields_ok_and_nok() -> anyhow::Result<()> {
    assert_eval(
        r#"
pattern test = {
    #[reason("Not bar")]
    foo: "bar",
    #[reason("Not baz")]
    bar: "baz"
}
"#,
        json!({
            "bar": "bar",
            "foo": "bar",
        }),
        json!({
            "name": "test::test",
            "severity": "error",
            "reason": "Because not all fields were satisfied",
            "rationale": [
                {
                    "name": "field:bar",
                    "severity": "error",
                    "reason": "Not baz",
                    "rationale": [{
                        "severity": "error",
                        "reason": "Not baz",
                    }]
                },
                {
                    "name": "field:foo",
                    "reason": "The input matches the expected constant value expected in the pattern",
                    "rationale": [{
                        "reason": "The input matches the expected constant value expected in the pattern",
                    }]
                },
            ]
        }),
    )
    .await;

    Ok(())
}

#[tokio::test]
async fn unused_field() -> anyhow::Result<()> {
    assert_eval(
        r#"
pattern test = {
    foo: "bar",
}
"#,
        json!({
           "bar": "baz",
        }),
        json!({
            "name": "test::test",
            "severity": "error",
            "reason": "Because not all fields were satisfied",
            "rationale": [
            ]
        }),
    )
    .await;

    Ok(())
}
