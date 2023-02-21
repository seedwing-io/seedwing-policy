use serde_json::Value;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct ResultViewProps {
    #[prop_or_default]
    pub rationale: String,
}

#[function_component(ResultView)]
pub fn result(props: &ResultViewProps) -> Html {
    // yes, we need to wrap it into a div (or some other element)
    let html = format!("<div>{}</div>", props.rationale.clone());
    html!(
        <div class="rationale">
            { Html::from_html_unchecked(html.into()) }
        </div>
    )
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct EvaluateRequest {
    name: String,
    policy: String,
    value: Value,
}

/// Do a remote evaluation with the server
pub async fn eval(policy: String, name: String, value: Value) -> Result<String, String> {
    let request = EvaluateRequest {
        name,
        policy,
        value,
    };

    let response = gloo_net::http::Request::post(&format!("/api/playground/v1alpha1/evaluate"))
        .json(&request)
        .map_err(|err| format!("Failed to encode request: {err}"))?
        .send()
        .await
        .map_err(|err| format!("Failed to send eval request: {err}"))?;

    response
        .text()
        .await
        .map_err(|err| format!("Failed to read response: {err}"))
}

/// Validate a remote policy
pub async fn validate(path: &str, value: Value) -> Result<String, String> {
    let response = gloo_net::http::Request::post(&format!("/api/policy/v1alpha1/{path}"))
        .json(&value)
        .map_err(|err| format!("Failed to encode request: {err}"))?
        .send()
        .await
        .map_err(|err| format!("Failed to send request: {err}"))?;

    let payload = response
        .text()
        .await
        .map_err(|err| format!("Failed to read response: {err}"))?;

    match (response.ok(), response.status()) {
        (true, _) | (false, 406) => Ok(payload),
        (false, _) => Err(format!(
            "{} {}: {payload}",
            response.status(),
            response.status_text()
        )),
    }
}
