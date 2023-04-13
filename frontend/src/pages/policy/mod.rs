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
use seedwing_policy_engine::runtime::metadata::{
    ComponentMetadata, PackageMetadata, PatternMetadata, SubpackageMetadata,
};
use seedwing_policy_engine::runtime::Example;
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

pub async fn fetch(path: &Vec<String>) -> Result<Option<ComponentMetadata>, String> {
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
        .map(|s| s.as_str().trim_end_matches(':'))
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
                <ComponentTitle base_path={parent.clone()} component={component.clone()}/>
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
        None => html!(<Title><Content>{ last(&parent) }</Content></Title>),
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
                { title }
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
    pub component: ComponentMetadata,
}

#[function_component(ComponentTitle)]
pub fn component_title(props: &ComponentProps) -> Html {
    let nav_path = props.base_path.join("::");
    let monitor = AppRoute::Monitor {
        path: nav_path.clone(),
    };
    let statistics = AppRoute::Statistics {
        path: nav_path.clone(),
    };

    html!(
        <>
        <Flex>
            <FlexItem>
                <Content>
                    {
                        match &props.component {
                            ComponentMetadata::Pattern(pattern) => html!(
                                <>
                                    <Title size={Size::XXXXLarge}>
                                        <Label color={Color::Blue} label={"T"} /> { " " }
                                        { render_full_type(pattern) }
                                        if pattern.metadata.unstable {
                                            {" "}<Label color={Color::Orange} compact=true label="unstable" />
                                        }
                                    </Title>
                                    <div>{ pattern.metadata.documentation.summary() }</div>
                                </>
                            ),
                            ComponentMetadata::Package(package) => html!(
                                <>
                                    <Title size={Size::XXXXLarge}>
                                        <Label color={Color::Blue} label={"M"} /> { " " }
                                        { last(&props.base_path) }
                                    </Title>
                                    <div>{ package.documentation.summary() }</div>
                                </>
                            ),
                        }
                    }
                </Content>
            </FlexItem>
            <FlexItem modifiers={[FlexModifier::Align(Alignment::Right)]}>
                <Link<AppRoute> target={monitor}>
                    <Button label="Monitor" variant={ButtonVariant::Secondary}/>
                </Link<AppRoute>>
                <Link<AppRoute> target={statistics}>
                    <Button label="Statistics" variant={ButtonVariant::Secondary}/>
                </Link<AppRoute>>
            </FlexItem>
        </Flex>
        </>)
}

#[function_component(Component)]
pub fn component(props: &ComponentProps) -> Html {
    html!(
        <>
            {match &props.component {
              ComponentMetadata::Pattern(pattern) => render_type(Rc::new(pattern.clone())),
              ComponentMetadata::Package(module) => render_module(props.base_path.clone(), module)
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
                if ! segment.is_empty() {
                    path.push_str("::");
                }

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

fn render_full_type(pattern: &PatternMetadata) -> Html {
    html!(<>
        {pattern.name.as_deref().unwrap_or_default()}
        if !pattern.parameters.is_empty() {
            {"<"}
            { for pattern.parameters.iter().map(|s|Html::from(s)) }
            {">"}
        }
    </>)
}

fn render_type(pattern: Rc<PatternMetadata>) -> Html {
    let path = pattern.path.as_deref().unwrap_or_default().to_string();
    let examples = pattern.examples.clone();

    html!(
        <>
            <Flex modifiers={[FlexModifier::Column.all(), FlexModifier::Row.md()]}>

                <Flex modifiers={[FlexModifier::Column, FlexModifier::Flex1]}>

                    {if let Some(deprecation) = &pattern.metadata.deprecation {
                        Some(html_nested!(
                            <FlexItem>
                                <Alert
                                    inline=true
                                    r#type={AlertType::Warning}
                                    title={format!("Deprecated{}", deprecation.since.as_deref().map(|s|format!(" since {}", s)).unwrap_or_default())}
                                >
                                    if let Some(reason) = &deprecation.reason { {reason} }
                                </Alert>
                            </FlexItem>
                        ))
                    } else { None }}

                    <FlexItem>
                        <Title level={Level::H2}> { "Documentation" } </Title>
                        <Asciidoc content={pattern.metadata.documentation.as_deref().unwrap_or_default().to_string()}/>
                    </FlexItem>
                    <FlexItem>
                        <Title level={Level::H2}> { "Definition" } </Title>
                        <ExpandableSection initially_open=true>
                            <Inner {pattern}/>
                        </ExpandableSection>
                    </FlexItem>
                </Flex>

                <FlexItem modifiers={[FlexModifier::Flex1]}>
                    <Experiment {path} {examples}/>
                </FlexItem>

            </Flex>

        </>
    )
}

fn render_module(base: Rc<Vec<String>>, module: &PackageMetadata) -> Html {
    let path = base.join("::");

    let packages_header = html_nested! {
        <TableHeader>
          <TableColumn label="Package"/>
          <TableColumn label="Summary"/>
        </TableHeader>
    };

    let package_entries = SharedTableModel::new(
        module
            .packages
            .iter()
            .map(|e| PackageRow(path.clone(), e.clone()))
            .collect(),
    );

    let patterns_header = html_nested! {
        <TableHeader>
          <TableColumn label="Pattern"/>
          <TableColumn label="Summary"/>
        </TableHeader>
    };

    let pattern_entries = SharedTableModel::new(
        module
            .patterns
            .iter()
            .map(|e| PatternRow(path.clone(), e.clone()))
            .collect(),
    );

    html!(
        <>
            <Flex space_items={[SpaceItems::Large]}>
                <FlexItem modifiers={[FlexModifier::Flex2]}>
                    if !package_entries.is_empty() {
                        <div class="pf-u-mb-xl">
                            <Table<SharedTableModel<PackageRow>> header={packages_header} entries={package_entries} mode={TableMode::Compact}/>
                        </div>
                    }
                    if !pattern_entries.is_empty() {
                        <div class="pf-u-mb-xl">
                            <Table<SharedTableModel<PatternRow>> header={patterns_header} entries={pattern_entries} mode={TableMode::Compact}/>
                        </div>
                    }
                </FlexItem>
                <FlexItem modifiers={[FlexModifier::Flex1]}>
                    { module.documentation.details() }
                </FlexItem>
            </Flex>
        </>
    )
}

#[derive(PartialEq)]
pub struct PackageRow(pub String, pub SubpackageMetadata);

impl TableEntryRenderer for PackageRow {
    fn render_cell(&self, context: &CellContext) -> Cell {
        match context.column {
            0 => {
                let path = format!("{}{name}::", self.0, name = self.1.name);
                html!(
                    <Link<AppRoute> target={AppRoute::Policy {path}}>{&self.1.name}</Link<AppRoute>>
                )
            }
            1 => html!(
                <p>{&self.1.documentation.summary()}</p>
            ),
            _ => html!(),
        }
        .into()
    }
}

#[derive(PartialEq)]
pub struct PatternRow(pub String, pub PatternMetadata);

impl TableEntryRenderer for PatternRow {
    fn render_cell(&self, context: &CellContext) -> Cell {
        match context.column {
            0 => {
                let path = format!("{}{name}", self.0, name=self.1.name.as_deref().unwrap_or(""));
                html!(
                    <Link<AppRoute> target={AppRoute::Policy {path}}>{&self.1.name.as_deref().unwrap_or("")}</Link<AppRoute>>
                )
            },
            1 => html!(
                <p>{&self.1.metadata.documentation.summary()}</p>
            ),
            _ => html!(),
        }.into()
    }
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
                            html!(<ResultView result={vec![result.clone()]}/>)
                        }
                        _ => html!(),
                    }
                }
                </PanelFooter>
            </Panel>
        </>
    )
}
