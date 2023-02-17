use crate::pages::AppRoute;
use gloo_net::http::Request;
use patternfly_yew::*;
use seedwing_policy_engine::runtime::{ComponentInformation, ModuleHandle, TypeInformation};
use seedwing_policy_frontend_asciidoctor::Asciidoc;
use std::rc::Rc;
use yew::prelude::*;
use yew_hooks::{use_async, UseAsyncState};
use yew_nested_router::components::Link;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct Props {
    pub path: AttrValue,
}

pub async fn fetch(path: &Vec<String>) -> Result<Option<ComponentInformation>, String> {
    log::info!("fetching: {path:?}");

    // FIXME: urlencode segments
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

/*
#[function_component(XRepository)]
pub fn x_repository(props: &Props) -> Html {
    let parent = use_memo(
        |path| path.split("::").map(|s| s.to_string()).collect::<Vec<_>>(),
        props.path.clone(),
    );

    let last = parent
        .last()
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .unwrap_or("Root")
        .to_string();

    html!(
        <>
        <PageSectionGroup
            sticky={[PageSectionSticky::Top]}
        >
            <PageSection r#type={PageSectionType::Breadcrumbs}>
                <Breadcrumbs {parent} />
            </PageSection>
            <PageSection variant={PageSectionVariant::Light}>
                <Title>{ last }</Title>
            </PageSection>
        </PageSectionGroup>
        <PageSection variant={PageSectionVariant::Light} fill=true>
            <RepositoryViewer ..props.clone()/>
        </PageSection>
        </>
    )
}
*/

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
            path.split("::").map(|s| s.to_string()).collect::<Vec<_>>()
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

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct ComponentProps {
    pub base_path: Rc<Vec<String>>,
    pub component: ComponentInformation,
}

#[function_component(ComponentTitle)]
pub fn component_title(props: &ComponentProps) -> Html {
    match &props.component {
        ComponentInformation::Type(r#type) => html!(
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
    match &props.component {
        ComponentInformation::Type(r#type) => render_type(r#type),
        ComponentInformation::Module(module) => render_module(props.base_path.clone(), module),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct BreadcrumbsProps {
    parent: Rc<Vec<String>>,
}

#[function_component(Breadcrumbs)]
fn render_breadcrumbs(props: &BreadcrumbsProps) -> Html {
    let mut path = String::new();

    log::info!("Path: {:?}", props.parent);

    let root = vec![String::new()];
    let bpath = root.iter().chain(props.parent.iter());

    html!(
        <Breadcrumb>
            { for bpath.enumerate()
                    .filter(|(n, segment)| *n == 0 || !segment.is_empty() )
                    .map(|(_, segment)|{

                path.push_str(&segment);
                path.push_str("::");

                let target = AppRoute::Repository { path: path.clone() };

                html_nested!(
                    <BreadcrumbRouterItem<AppRoute>
                        to={target}
                    >
                        { if segment.is_empty() {
                            "Root"
                        } else {
                            &segment
                        } }
                    </BreadcrumbRouterItem<AppRoute>>
                )
            })}
        </Breadcrumb>
    )
}

fn render_full_type(r#type: &TypeInformation) -> Html {
    html!(<>
        {r#type.name.as_deref().unwrap_or_default()}
        if !r#type.parameters.is_empty() {
            {"<"}
            { for r#type.parameters.iter().map(|s|Html::from(s)) }
            {">"}
        }
    </>)
}

fn render_type(r#type: &TypeInformation) -> Html {
    html!(
        <>
            <Content>
                <dl>
                    <dt>{"Name"}</dt>
                    <dd>
                        { render_full_type(r#type) }
                    </dd>
                </dl>
                <Asciidoc content={r#type.documentation.as_deref().unwrap_or_default().to_string()}/>
            </Content>
        </>
    )
}

fn render_module(base: Rc<Vec<String>>, module: &ModuleHandle) -> Html {
    let path = base.join("::");

    html!(
        <ul>
            { for module.modules.iter().map(|module| {
                let path = format!("{path}{module}::");
                html!(<li key={module.clone()}><Link<AppRoute> target={AppRoute::Repository {path}}>{&module}</Link<AppRoute>></li>)
            })}
            { for module.types.iter().map(|r#type| {
                let path = format!("{path}{type}");
                html!(<li key={r#type.clone()}><Link<AppRoute> target={AppRoute::Repository {path}}>{&r#type}</Link<AppRoute>></li>)
            })}
        </ul>
    )
}
