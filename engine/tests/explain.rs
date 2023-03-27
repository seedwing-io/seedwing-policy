use serde_json::json;

mod common;

use common::*;

#[tokio::test]
async fn simple_pattern() -> anyhow::Result<()> {
    assert_eval(
        r#"
#[explain("Not foo")]
pattern test = {}
"#,
        json!(false),
        json!({
            "name": "test::test",
            "failed": "Not foo",
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
  #[explain("Not baz")]
  bar: "baz"
}
"#,
        json!({"bar": "bar"}),
        json!({
            "name": "test::test",
            "failed": "because not all fields were satisfied",
            "rationale": [
                {
                    "name": "field:bar",
                    "failed": "Not baz",
                    "rationale": [{
                        "failed": "Not baz"
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
    #[explain("Not bar")]
    foo: "bar",
    #[explain("Not baz")]
    bar: "baz"
}
"#,
        json!({
            "bar": "bar",
            "foo": "bar",
        }),
        json!({
            "name": "test::test",
            "failed": "because not all fields were satisfied",
            "rationale": [
                {
                    "name": "field:bar",
                    "failed": "Not baz",
                    "rationale": [{
                        "failed": "Not baz",
                    }]
                },
                {
                    "name": "field:foo",
                    "rationale": [
                        {}
                    ]
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
            "failed": "because not all fields were satisfied",
            "rationale": [
            ]
        }),
    )
    .await;

    Ok(())
}
