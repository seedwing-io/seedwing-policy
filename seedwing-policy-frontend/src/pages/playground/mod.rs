use crate::pages::playground::marker::{ByteRange, MarkerData};
use editor::Editor;
use monaco::sys::MarkerSeverity;
use patternfly_yew::*;
use seedwing_policy_engine::{lang::builder::Builder, runtime::sources::Ephemeral};
use serde_json::Value;
use yew::prelude::*;
use yew_hooks::{use_async, UseAsyncState};

mod editor;
mod marker;

const INITIAL_POLICY: &str = r#"pattern dog = {
    name: string,
    trained: boolean
}"#;
const INITIAL_VALUE: &str = r#"name: goodboy
trained: true"#;

fn test_compile(value: &str) -> Result<(), Vec<MarkerData>> {
    let source = Ephemeral::new("test", value);

    let rope = ropey::Rope::from_str(value);

    let mut builder = Builder::new();
    builder.build(source.iter()).map_err(|err| {
        err.into_iter()
            .map(|err| {
                let range: std::ops::Range<marker::Position> =
                    ByteRange(&rope, err.span()).try_into().unwrap_or_default();

                MarkerData::new(err.to_string(), MarkerSeverity::Error, range)
            })
            .collect::<Vec<_>>()
    })?;
    //builder.finish().await?;

    Ok(())
}

#[function_component(Playground)]
pub fn playground() -> Html {
    let markers = use_state_eq(|| Vec::<MarkerData>::new());

    let pattern = use_state_eq(String::new);

    let on_pattern_change = {
        let pattern = pattern.clone();
        let markers = markers.clone();
        use_callback(
            move |text: String, _| {
                pattern.set(text.clone());
                match test_compile(&text) {
                    Err(err) => {
                        markers.set(err);
                    }
                    Ok(()) => {
                        markers.set(vec![]);
                    }
                }
            },
            (),
        )
    };

    let policy_editor = use_memo(
        |markers| {
            use std::ops::Deref;
            html!(<Editor language="dogma" initial_content={INITIAL_POLICY} on_change={on_pattern_change} markers={markers.deref().clone()}/>)
        },
        markers,
    );

    let value = use_state_eq(|| Value::Null);
    let on_value_change = use_callback(
        |text: String, value| {
            value.set(serde_yaml::from_str(&text).unwrap_or(Value::Null));
        },
        value.clone(),
    );

    let value_editor = use_memo(
        |()| html!(<Editor initial_content={INITIAL_VALUE} on_change={on_value_change} language="yaml"/>),
        (),
    );

    html!(
        <>
        <PageSection>
            <Title>{"Playground"}</Title>
        </PageSection>

        <PageSection variant={PageSectionVariant::Light}>
        <div class="playground">
        <Flex>
            <FlexItem modifiers={[FlexModifier::Flex1]}>
                <Flex modifiers={[FlexModifier::Column]}>
                    <FlexItem modifiers={[FlexModifier::Flex1]}>
                        <Title>{"Pattern"}</Title>
                        { (*policy_editor).clone() }
                    </FlexItem>
                    <FlexItem modifiers={[FlexModifier::Flex1]}>
                        <Title>{"Data"}</Title>
                        { (*value_editor).clone() }
                    </FlexItem>
                </Flex>
            </FlexItem>
            <FlexItem modifiers={[FlexModifier::Flex2]}>
                <EvalView pattern={(*pattern).clone()} value={(*value).clone()} />
            </FlexItem>
        </Flex>
        </div>
        </PageSection>
        </>
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct EvalViewProps {
    pattern: String,
    value: Value,
}

#[function_component(EvalView)]
pub fn eval_view(props: &EvalViewProps) -> Html {
    let name = use_state_eq(|| "dog".to_string());

    let eval = {
        let pattern = props.pattern.clone();
        let value = props.value.clone();
        let name = (*name).clone();
        use_async(async move { eval(pattern, name, value).await })
    };

    let onclick = {
        let eval = eval.clone();
        Callback::from(move |_| {
            eval.run();
        })
    };

    let onchange = {
        //let name = name.clone();
        use_callback(
            move |text, name| {
                name.set(text);
            },
            name,
        )
    };

    html!(
        <>
        <Title>
            {"Evaluate"}
        </Title>
        <Toolbar>
            <ToolbarItem>
                <TextInput {onchange} value="dog" required=true placeholder="Name of the pattern to evaluate" />
            </ToolbarItem>
            <ToolbarItem>
                <Button label="Evaluate" disabled={eval.loading} variant={Variant::Primary} {onclick}/>
            </ToolbarItem>
        </Toolbar>
        {
            match &*eval {
                UseAsyncState { loading: true, .. } => {
                    html!("Loading...")
                }
                UseAsyncState {
                    error: Some(err), ..
                } => {
                    html!(format!("Failed: {err}"))
                }
                UseAsyncState {
                    data: Some(rationale),
                    ..
                } => {
                    html!(
                        <>
                            <ResultView rationale={rationale.clone()}/>
                        </>
                    )
                }
                _ => html!("Click eval"),
            }
        }
        </>
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct ResultViewProps {
    #[prop_or_default]
    rationale: String,
}

#[function_component(ResultView)]
pub fn result(props: &ResultViewProps) -> Html {
    // yes, we need to wrap it into a div (or some other element)
    let html = format!("<div>{}</div>", props.rationale.clone());
    log::info!("Html: {html}");
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
async fn eval(policy: String, name: String, value: Value) -> Result<String, String> {
    let request = EvaluateRequest {
        name,
        policy,
        value,
    };

    log::info!("Request: {request:?}");

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
