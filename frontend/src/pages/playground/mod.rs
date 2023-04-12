mod store;

use crate::common::editor::Generation;
use crate::common::{
    editor::{self, ByteRange, Editor, MarkerData},
    eval::{eval, ResultView},
};
use monaco::{
    sys::{
        languages::{CommentRule, LanguageConfiguration},
        MarkerSeverity,
    },
    yew::CodeEditorLink,
};
use monaco_editor_textmate_web::prelude::*;
use patternfly_yew::{next::TextInput, prelude::*};
use seedwing_policy_engine::{
    lang::builder::Builder,
    runtime::{sources::Ephemeral, Response},
};
use serde_json::Value;
use store::ExampleData;
use wasm_bindgen::JsCast;
use yew::prelude::*;
use yew_hooks::{use_async, UseAsyncState};

const DOGMA_TEXTMATE: &str = include_str!("../../../textmate/dogma.tmLanguage.json");
const DOGMA_LANGUAGE_ID: &str = "dogma";
const DOGMA_SCOPE_ID: &str = "source.dog";

fn test_compile(value: &str) -> Result<(), Vec<MarkerData>> {
    let source = Ephemeral::new("test", value);

    let rope = ropey::Rope::from_str(value);

    let mut builder = Builder::new();
    builder.build(source.iter()).map_err(|err| {
        err.into_iter()
            .map(|err| {
                let range: std::ops::Range<editor::Position> =
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
    let example = use_state(|| Generation::from(ExampleData::load_default()));
    let initial_name = use_memo(
        |example| (*example).as_ref().map(|e| e.policy.clone()),
        example.clone(),
    );

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
        |(markers, example)| {
            let on_editor_created = Callback::from(|editor: CodeEditorLink| {
                // ensure language is registered

                register_language(DOGMA_LANGUAGE_ID);

                // and configured

                let comments: CommentRule = js_sys::Object::new().unchecked_into();
                comments.set_line_comment(Some("//"));

                let lang: LanguageConfiguration = js_sys::Object::new().unchecked_into();
                lang.set_comments(Some(&comments));
                monaco::sys::languages::set_language_configuration(DOGMA_LANGUAGE_ID, &lang);

                // set textmate provider

                set_textmate_provider(
                    &editor,
                    GrammarDefinition::new("json", DOGMA_TEXTMATE),
                    DOGMA_LANGUAGE_ID,
                    DOGMA_SCOPE_ID,
                );
            });

            use std::ops::Deref;
            html!(<Editor<Generation<String>>
                language={DOGMA_LANGUAGE_ID}
                initial_content={example.as_ref().map(|e|e.definition.clone())}
                markers={markers.deref().clone()}
                on_change={on_pattern_change}
                {on_editor_created}
            />)
        },
        (markers, example.clone()),
    );

    let value = use_state_eq(|| Value::Null);
    let on_value_change = use_callback(
        |text: String, value| {
            value.set(serde_yaml::from_str(&text).unwrap_or(Value::Null));
        },
        value.clone(),
    );

    let value_editor = use_memo(
        |example| html!(<Editor<Generation<String>> initial_content={(*example).as_ref().map(|e|e.value.clone())} on_change={on_value_change} language="yaml"/>),
        example.clone(),
    );

    // eval section

    let name = use_state_eq(|| example.policy.clone());

    let eval = {
        let pattern = pattern.clone();
        let value = value.clone();
        let name = (*name).clone();
        use_async(async move { eval((*pattern).clone(), name, (*value).clone()).await })
    };

    let onclick = {
        let eval = eval.clone();
        Callback::from(move |_| {
            eval.run();
        })
    };

    let onchange = {
        use_callback(
            move |text: String, name| {
                name.set(text);
            },
            name.clone(),
        )
    };

    let policy_name_help = html_nested!(
        <PopoverBody header={html!("Policy Name")}>
            <Content>
                <p>{"Enter the name of a policy to evaluate."}</p>
                <p>{"Most likely, you want to enter the name of a pattern from the left-hand side box here."}</p>
            </Content>
        </PopoverBody>
    );

    // example storage

    let store_cb = {
        let pattern = pattern.clone();
        let value = value.clone();
        let value = serde_yaml::to_string(&*value).unwrap_or_default();
        let policy_name = name.clone();
        Callback::from(move |()| {
            let example = ExampleData {
                definition: (*pattern).clone(),
                value: value.clone(),
                policy: (*policy_name).clone(),
            };
            ExampleData::store_default(example);
        })
    };

    let reset_cb = {
        let example = example.clone();
        Callback::from(move |()| {
            let data = Generation::from(ExampleData::load_default());
            example.set(data);
        })
    };

    let clear_cb = {
        let example = example.clone();
        Callback::from(move |()| {
            ExampleData::clear_default();
            example.set(Generation::from(ExampleData::default()));
        })
    };

    // render

    html!(
        <>
        <PageSection variant={PageSectionVariant::Light} sticky={[PageSectionSticky::Top]}>
            <Flex>
                <FlexItem modifiers={[FlexModifier::Grow]}>
                    <Content>
                        <Title size={Size::XXXXLarge}>
                            {"Playground"}
                        </Title>
                    </Content>
                </FlexItem>
                <FlexItem modifiers={[FlexModifier::Align(Alignment::End)]}>
                    <Dropdown position={Position::Right} toggle={html!(<DropdownToggle text="Examples"/>)}>
                        <DropdownItem description="Reset current values to the stored default" onclick={reset_cb}>{ "Reset" }</DropdownItem>
                        <ListDivider/>
                        <DropdownItemGroup title="Storage">
                            <DropdownItem description="Store the current configuration as the new default" onclick={store_cb}>{ "Store as default" }</DropdownItem>
                            <DropdownItem description="Revert default to the system default" onclick={clear_cb}>{ "Clear default" }</DropdownItem>
                        </DropdownItemGroup>
                    </Dropdown>
                </FlexItem>
            </Flex>
            <Content>
                { Html::from_html_unchecked(r#"<p>The <b>playground</b> is a place to interactively try out policies</p>"#.into()) }
            </Content>
        </PageSection>

        <PageSection variant={PageSectionVariant::Light} fill=true>
        <div class="playground">
        <Flex>
            <FlexItem modifiers={[FlexModifier::Flex1]}>
                <Flex modifiers={[FlexModifier::Column]}>
                    <FlexItem modifiers={[FlexModifier::Flex1]}>
                        <Title>{"Pattern"}</Title>
                        { (*policy_editor).clone() }
                    </FlexItem>
                    <FlexItem modifiers={[FlexModifier::Flex1]}>
                        <Title>{"Data "} <Label label="YAML"/></Title>
                        { (*value_editor).clone() }
                    </FlexItem>
                </Flex>
            </FlexItem>

            <FlexItem modifiers={[FlexModifier::Flex2]}>
                <Title>
                    {"Evaluate"}
                </Title>
                <Toolbar>
                    <ToolbarItem>
                        <Form horizontal={[FormHorizontal.all()]}>
                            <FormGroup label="Policy" required=true
                                label_icon={LabelIcon::Help(policy_name_help)}
                            >
                                <TextInput {onchange}
                                    value={(*initial_name).clone()}
                                    required=true
                                    placeholder="Name of the pattern to evaluate"
                                />
                            </FormGroup>
                        </Form>
                    </ToolbarItem>
                    <ToolbarItem>
                        <Button label="Evaluate" disabled={eval.loading} variant={ButtonVariant::Primary} {onclick}/>
                    </ToolbarItem>
                </Toolbar>
                <EvalView eval={(*eval).clone()} />
            </FlexItem>

        </Flex>
        </div>
        </PageSection>
        </>
    )
}

#[derive(PartialEq, Eq, Properties)]
pub struct EvalViewProps {
    pub eval: UseAsyncState<Response, String>,
}

#[function_component(EvalView)]
pub fn eval_view(props: &EvalViewProps) -> Html {
    html!(
        <>
        {
            match &props.eval {
                UseAsyncState { loading: true, .. } => {
                    html!("Loading...")
                }
                UseAsyncState {
                    error: Some(err), ..
                } => {
                    html!(format!("Failed: {err}"))
                }
                UseAsyncState {
                    data: Some(result),
                    ..
                } => {
                    html!(
                        <ResultView result={result.clone()}/>
                    )
                }
                _ => html!(""),
            }
        }
        </>
    )
}
