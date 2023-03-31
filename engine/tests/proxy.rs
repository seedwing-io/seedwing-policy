use seedwing_policy_engine::lang::Severity;
use seedwing_policy_engine::runtime::Response;
use serde_json::json;

mod common;

/// Test the proxy case with the jdom example
///
/// NOTE: This test is ignored by default as it is an online test which might fail and
/// return different data in the future.
#[tokio::test]
#[ignore]
async fn test_jdom() {
    let input = json!({
      "hash": "02bd61a725e8af9b0176b43bf29816d0c748b8ab951385bd127be37489325a0a",
      "purl": "pkg:maven/org.jdom/jdom@1.1.3?type=jar&repository_url=https%3A%2F%2Frepo.maven.apache.org%2Fmaven2",
      "url": "https://repo.maven.apache.org/maven2/org/jdom/jdom/1.1.3/jdom-1.1.3.jar"
    });

    let result = common::eval(include_str!("proxy.dog"), "not-affected", input).await;
    assert_eq!(result.severity(), Severity::Error);

    let response = Response::new(&result);
    println!("{}", serde_json::to_string_pretty(&response).unwrap());

    let response = response.collapse(Severity::Error);
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
}
