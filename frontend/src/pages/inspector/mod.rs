use crate::common::{editor::Editor, eval::ResultTreeView};
use base64::Engine;
use patternfly_yew::prelude::*;
use seedwing_policy_engine::runtime::Response;
use yew::prelude::*;

const INITIAL_VALUE: &str = r#"{
    "name": {
      "pattern": "inspector"
    },
    "input": "If you paste the JSON output of an evaluation into the editor, you can in drill into details on the 'Inspect' tab.",
    "reason": "This is just an example"
}"#;

#[function_component(Inspector)]
pub fn inspector() -> Html {
    let result = use_state_eq(|| Ok(vec![Response::default()]));

    let on_change = use_callback(
        |text: String, value| {
            value.set(serde_json::from_str(&text).map_err(|err| err.to_string()));
        },
        result.clone(),
    );

    let initial = use_memo(
        |()| {
            // try getting a result from a string, or silently revert to the default content
            gloo_utils::window()
                .location()
                .hash()
                .ok()
                .filter(|s| !s.is_empty())
                .and_then(|h| {
                    base64::prelude::BASE64_STANDARD
                        .decode(h.trim_start_matches('#'))
                        .ok()
                })
                .and_then(|s| String::from_utf8(s).ok())
                .unwrap_or_else(|| INITIAL_VALUE.to_string())
        },
        (),
    );

    let editor = use_memo(
        |initial| html!(<Editor initial_content={(**initial).clone()} on_change={on_change} language="json"/>),
        initial.clone(),
    );

    let tab = use_state_eq(|| 0);
    let onselect = {
        let tab = tab.clone();
        Callback::from(move |index: usize| {
            tab.set(index);
        })
    };

    let body = html_nested!(
        <PopoverBody
            header={html!("Usage")}
        >
            {"You can inspect the result of a policy evaluation by copying the JSON output and pasting it into the editor in the 'Paste' tab. If the content is a valid result, the 'Inspect' tab will show a result tree."}
        </PopoverBody>
    );

    html!(
        <>
            <PageSection variant={PageSectionVariant::Light}>
                <Content>
                    <Title size={Size::XXXXLarge}>{ "Inspector" }</Title>
                    <p>
                    { Html::from_html_unchecked(r#"<span>The <b>inspector</b> can visualize the outcome of a policy evaluation in more detail</span> "#.into()) }
                    <Popover toggle_by_onclick=true target={html!(<span class="sw-help-icon"> {" "} {Icon::QuestionCircle} </span>)} {body} />
                    </p>
                </Content>
            </PageSection>

            <PageSection r#type={PageSectionType::Tabs} variant={PageSectionVariant::Light} sticky={[PageSectionSticky::Top]}>
                <Tabs inset={TabInset::Page} detached=true {onselect}>
                    <Tab label="Paste"/>
                    <Tab label="Inspect"/>
                </Tabs>
            </PageSection>

            <PageSection hidden={*tab != 0} id="inspector" fill={PageSectionFill::Fill}>
                {(*editor).clone()}
            </PageSection>

            <PageSection hidden={*tab != 1} fill={PageSectionFill::Fill}>
                {
                    match &*result {
                        Ok(result) => html!(<ResultTreeView result={result.clone()}/>),
                        Err(err) => html!(
                            <CodeBlock>
                                <CodeBlockCode>
                                    {err}
                                </CodeBlockCode>
                            </CodeBlock>
                        )
                    }
                }
            </PageSection>

        </>
    )
}
