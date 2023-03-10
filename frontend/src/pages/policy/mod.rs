use crate::{
    common::{
        editor::Editor,
        eval::{validate, ResultView},
    },
    pages::AppRoute,
};
use asciidoctor_web::yew::Asciidoc;
use gloo_net::http::Request;
use inner::Inner;
use patternfly_yew::prelude::*;
use seedwing_policy_engine::{
    info::{ComponentInformation, PatternInformation},
    runtime::{Example, ModuleHandle},
};
use serde_json::Value;
use std::fmt::Formatter;
use std::rc::Rc;
use yew::prelude::*;
use yew_hooks::{use_async, UseAsyncState};
use yew_nested_router::components::Link;

mod inner;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct Props {
    pub path: AttrValue,
}

pub async fn fetch(path: &Vec<String>) -> Result<Option<ComponentInformation>, String> {
    let path = path.join("/");

    let response = Request::get(&format!("/api/policy/v1alpha1/{}", path))
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if response.status() == 404 {
        Ok(None)
    } else {
        Ok(Some(response.json().await.map_err(|err| err.to_string())?))
    }
}

fn last(parent: &Vec<String>) -> String {
    parent
        .iter()
        .rev()
        .filter(|s| !s.is_empty())
        .next()
        .map(|s| s.as_str())
        .unwrap_or("Root")
        .to_string()
}

#[function_component(Repository)]
pub fn repository(props: &Props) -> Html {
    let parent = use_memo(
        |path| {
            let path = path.trim_start_matches(":");
            vec![path.to_string()]
        },
        props.path.clone(),
    );

    let fetch_path = parent.clone();
    let state = use_async(async move { fetch(&fetch_path).await });

    {
        let state = state.clone();
        use_effect_with_deps(
            move |_| {
                state.run();
            },
            parent.clone(),
        );
    }

    let (title, main) = match &*state {
        UseAsyncState { loading: true, .. } => (None, html!({ "Loading..." })),
        UseAsyncState {
            loading: false,
            error: Some(error),
            ..
        } => (None, html!(<> {"Failed: "} {error} </>)),

        UseAsyncState {
            data: Some(Some(component)),
            ..
        } => (
            Some(html!(
                <Title>
                    <ComponentTitle base_path={parent.clone()} component={component.clone()}/>
                </Title>
            )),
            html!(<Component base_path={parent.clone()} component={component.clone()}/>),
        ),
        UseAsyncState {
            data: Some(None), ..
        } => (None, html!(<>{"Component not found: "} {&props.path}</>)),
        _ => (None, html!("Unknown state")),
    };

    let title = match title {
        Some(title) => title,
        None => last(&parent).into(),
    };

    html!(
        <>
        <PageSectionGroup
            sticky={[PageSectionSticky::Top]}
        >
            <PageSection r#type={PageSectionType::Breadcrumbs}>
                <Breadcrumbs {parent} />
            </PageSection>
            <PageSection variant={PageSectionVariant::Light}>
                <Title>
                    <Content> { title } </Content>
                </Title>
            </PageSection>
        </PageSectionGroup>
        <PageSection variant={PageSectionVariant::Light} fill=true>
            { main }
        </PageSection>
        </>
    )
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct ComponentProps {
    pub base_path: Rc<Vec<String>>,
    pub component: ComponentInformation,
}

#[function_component(ComponentTitle)]
pub fn component_title(props: &ComponentProps) -> Html {
    match &props.component {
        ComponentInformation::Pattern(r#type) => html!(
            <>
                <Label color={Color::Blue} label={"T"} /> { " " }
                { render_full_type(r#type) }
            </>
        ),
        ComponentInformation::Module(_module) => {
            html!(
                <>
                    <Label color={Color::Blue} label={"M"} /> { " " }
                    { last(&props.base_path) }
                </>
            )
        }
    }
}

#[function_component(Component)]
pub fn component(props: &ComponentProps) -> Html {
    let nav_path = props.base_path.join("::");
    let monitor = AppRoute::Monitor {
        path: nav_path.clone(),
    };
    let statistics = AppRoute::Statistics {
        path: nav_path.clone(),
    };

    html!(
        <>
          <Toolbar>
              <ToolbarItem>
                <Link<AppRoute> target={monitor}>
                  <Button label="Monitor" variant={ButtonVariant::Secondary}/>
                </Link<AppRoute>>
              </ToolbarItem>
              <ToolbarItem>
                <Link<AppRoute> target={statistics}>
                  <Button label="Statistics" variant={ButtonVariant::Secondary}/>
                </Link<AppRoute>>
              </ToolbarItem>
          </Toolbar>
          {match &props.component {
              ComponentInformation::Pattern(r#type) => render_type(Rc::new(r#type.clone())),
              ComponentInformation::Module(module) => render_module(props.base_path.clone(), module),
          }}
        </>
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct BreadcrumbsProps {
    pub(crate) parent: Rc<Vec<String>>,
}

#[function_component(Breadcrumbs)]
fn render_breadcrumbs(props: &BreadcrumbsProps) -> Html {
    let mut path = String::new();

    let root = vec![String::new()];
    let bpath = root.iter().cloned().chain(
        props
            .parent
            .iter()
            .flat_map(|seg| seg.split("::").map(|e| e.to_string())),
    );

    html!(
        <Breadcrumb>
            { for bpath.enumerate()
                    .filter(|(n, segment)| *n == 0 || !segment.is_empty() )
                    .map(|(_, segment)|{

                path.push_str(&segment);
                path.push_str("::");

                let target = AppRoute::Policy { path: path.clone() };

                html_nested!(
                    <BreadcrumbRouterItem<AppRoute>
                        to={target}
                    >
                        { if segment.is_empty() {
                            "Library"
                        } else {
                            &segment
                        } }
                    </BreadcrumbRouterItem<AppRoute>>
                )
            })}
        </Breadcrumb>
    )
}

fn render_full_type(r#type: &PatternInformation) -> Html {
    html!(<>
        {r#type.name.as_deref().unwrap_or_default()}
        if !r#type.parameters.is_empty() {
            {"<"}
            { for r#type.parameters.iter().map(|s|Html::from(s)) }
            {">"}
        }
    </>)
}

fn render_type(r#type: Rc<PatternInformation>) -> Html {
    let path = r#type.name.as_deref().unwrap_or_default().to_string();
    let examples = r#type.examples.clone();

    html!(
        <>
            <Content>
                <dl>
                    <dt>{"Name"}</dt>
                    <dd>
                        { render_full_type(&r#type) }
                    </dd>
                </dl>
            </Content>

            <Flex>

                <FlexItem modifiers={[FlexModifier::Flex1]}>
                    <Title level={Level::H2}> { "Documentation" } </Title>
                    <ExpandableSection initial_state=true>
                        <Asciidoc content={r#type.documentation.as_deref().unwrap_or_default().to_string()}/>
                    </ExpandableSection>
                    <Title level={Level::H2}> { "Definition" } </Title>
                    <ExpandableSection>
                        <Inner {r#type}/>
                    </ExpandableSection>
                </FlexItem>

                <FlexItem modifiers={[FlexModifier::Flex1]}>
                    <Experiment {path} {examples}/>
                </FlexItem>

            </Flex>

        </>
    )
}

fn render_module(base: Rc<Vec<String>>, module: &ModuleHandle) -> Html {
    let path = base.join("::");

    html!(
        <>
        if !module.modules.is_empty() {
            <PageSection variant={PageSectionVariant::Light}>
                <Content>
                    <Title size={Size::XXLarge}>{"Modules"}</Title>
                    <ul>
                        { for module.modules.iter().map(|module| {
                            let path = format!("{path}{module}::");
                            html!(<li key={module.clone()}><Link<AppRoute> target={AppRoute::Policy {path}}>{&module}</Link<AppRoute>></li>)
                        })}
                    </ul>
                </Content>
            </PageSection>
        }
        if !module.types.is_empty() {
            <PageSection variant={PageSectionVariant::Light}>
                    <Content>
                        <Title size={Size::XXLarge}>{"Patterns"}</Title>
                        <ul>
                            { for module.types.iter().map(|r#type| {
                                let path = format!("{path}{type}");
                                html!(<li key={r#type.clone()}><Link<AppRoute> target={AppRoute::Policy {path}}>{&r#type}</Link<AppRoute>></li>)
                            })}
                        </ul>
                    </Content>
            </PageSection>
        }
        </>
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExampleEntry(pub Example);

impl std::fmt::Display for ExampleEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.summary.as_deref().unwrap_or(self.0.name.as_str()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct ExperimentProperties {
    pub examples: Vec<Example>,
    pub path: String,
}

#[function_component(Experiment)]
pub fn experiment(props: &ExperimentProperties) -> Html {
    let first = props
        .examples
        .first()
        .cloned()
        .map(ExampleEntry)
        .into_iter()
        .collect::<Vec<_>>();

    let value = use_state_eq(|| Value::Null);
    let on_change = use_callback(
        |text: String, value| {
            value.set(serde_yaml::from_str(&text).unwrap_or(Value::Null));
        },
        value.clone(),
    );

    // editor

    let initial_content = use_state_eq(|| {
        first
            .first()
            .as_ref()
            .and_then(|ex| serde_json::to_string_pretty(&ex.0.value).ok())
            .unwrap_or_default()
    });

    let editor = use_memo(
        |initial_content| html!(<Editor initial_content={initial_content.clone()} {on_change} language="yaml"/>),
        (*initial_content).clone(),
    );

    let eval = {
        let value = value.clone();
        let path = props.path.clone();
        use_async(async move { validate(&path, (*value).clone()).await })
    };

    let onclick = {
        let eval = eval.clone();
        Callback::from(move |_| {
            eval.run();
        })
    };

    // toolbar

    let mut toolbar = vec![];
    toolbar.push(html_nested!(
        <ToolbarItem>
            <Title level={Level::H2}> { "Experiment" } </Title>
        </ToolbarItem>));
    if !props.examples.is_empty() {
        let initial_content = initial_content.clone();
        let onselect = Callback::from(move |example: ExampleEntry| {
            // set editor
            if let Ok(value) = serde_json::to_string_pretty(&example.0.value) {
                initial_content.set(value);
            }
        });
        toolbar.push(html_nested!(<ToolbarItem>
            <Select<ExampleEntry>
                initial_selection={first}
                variant={SelectVariant::Single(onselect)}
            >
            { for props.examples.iter().map(|example|{
                html_nested!(<SelectOption<ExampleEntry>
                    value={ExampleEntry(example.clone())}
                    id={example.name.clone()}
                    description={example.description.clone()}
                />)
            })}
            </Select<ExampleEntry>>
        </ToolbarItem>));
    }
    toolbar.push(html_nested!(
        <ToolbarItem modifiers={[ToolbarElementModifier::Right]}>
            <Button label="POST" variant={ButtonVariant::Secondary} {onclick} disabled={eval.loading} />
        </ToolbarItem>
    ));

    // main

    html!(
        <>
            <Toolbar>{ for toolbar.into_iter() }</Toolbar>
            <Panel>
                <PanelMain>
                    <div class="experiment">
                    { (*editor).clone() }
                    </div>
                </PanelMain>
                <PanelFooter>
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
                            data: Some(result),
                            ..
                        } => {
                            html!(<ResultView result={result.clone()}/>)
                        }
                        _ => html!(),
                    }
                }
                </PanelFooter>
            </Panel>
        </>
    )
}
