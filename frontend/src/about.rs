use gloo_net::http::Request;
use patternfly_yew::prelude::*;
use yew::prelude::*;
use yew_hooks::{use_async_with_options, UseAsyncOptions};

pub async fn server_version() -> Result<String, String> {
    if let Ok(response) = Request::get("/api/version").send().await {
        if response.status() == 200 {
            match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    if let Some(Some(version)) = data.get("version").map(|v| v.as_str()) {
                        return Ok(version.to_string());
                    }
                }
                Err(e) => {
                    log::warn!("Error fetching server version: {:?}", e);
                }
            }
        }
    }
    Ok("unknown".to_string())
}

#[function_component(About)]
pub fn about() -> Html {
    let state = { use_async_with_options(server_version(), UseAsyncOptions::enable_auto()) };

    let client_version = seedwing_policy_engine::version();
    html!(
        <Bullseye plain=true>
            <patternfly_yew::prelude::About
                brand_src="images/logo-inverted.png"
                brand_alt="Seedwing logo"
                title="Seedwing Policy"
                strapline={html!("Copyright © 2023 Red Hat, Inc. and the Seedwing Project")}
                hero_style=r#"
--pf-c-about-modal-box__hero--lg--BackgroundImage: url("https://www.patternfly.org/assets/images/pfbg_992@2x.jpg");
--pf-c-about-modal-box__hero--sm--BackgroundImage: url("https://www.patternfly.org/assets/images/pfbg_992.jpg");
"#
            >
                <Content>
                    <dl style="width: 100%">
                        <dt>{ "Server Version" }</dt>
                        <dd>{
                            if state.loading {
                                html!{ "loading..." }
                            } else {
                                html! {
                                    state.data.as_ref().unwrap_or(&String::new())
                                }
                            }
                        }</dd>
                        <dt>{ "Client Version" }</dt>
                        <dd>{ client_version }</dd>
                        <dt>{ "License" }</dt>
                        <dd>{ env!("CARGO_PKG_LICENSE") }</dd>
                    </dl>
                </Content>
            </patternfly_yew::prelude::About>
        </Bullseye>
    )
}
