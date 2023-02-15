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

#[function_component(Repository)]
pub fn repository(props: &Props) -> Html {
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
        <PageSection variant={PageSectionVariant::Light}>
            <RepositoryViewer ..props.clone()/>
        </PageSection>
        </>
    )
}

#[function_component(RepositoryViewer)]
pub fn repository_viewer(props: &Props) -> Html {
    let path: Rc<Vec<String>> = Rc::new(props.path.split("::").map(|s| s.to_string()).collect());

    let fetch_path = path.clone();
    let state = use_async(async move { fetch(&fetch_path).await });

    {
        let state = state.clone();
        use_effect_with_deps(
            move |_| {
                state.run();
            },
            path.clone(),
        );
    }

    match &*state {
        UseAsyncState { loading: true, .. } => html!({ "Loading..." }),
        UseAsyncState {
            loading: false,
            error: Some(error),
            ..
        } => html!(<> {"Failed: "} {error} </>),
        UseAsyncState {
            data: Some(Some(component)),
            ..
        } => html!(<Component base_path={path.clone()} component={component.clone()}/>),
        UseAsyncState {
            data: Some(None), ..
        } => html!(<>{"Component not found: "} {&props.path}</>),
        _ => html!("Unknown state"),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct ComponentProps {
    pub base_path: Rc<Vec<String>>,
    pub component: ComponentInformation,
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
    let mut breadcrumbs: Vec<Html> = vec![];
    let mut path = String::new();

    log::info!("Path: {:?}", props.parent);

    for (n, segment) in props.parent.iter().enumerate() {
        if n > 0 {
            path.push_str("::");
        }
        path.push_str(&segment);
        let target = AppRoute::Repository { path: path.clone() };
        breadcrumbs.push(html!(
            <Link<AppRoute> {target}>
                { if segment.is_empty() {
                    "Root"
                } else {
                    &segment
                } }
            </Link<AppRoute>>
        ));
    }

    let last = breadcrumbs.len() - 1;

    html!(
        <>
        <nav class="pf-c-breadcrumb">
            <ol class="pf-c-breadcrumb__list">
               { for breadcrumbs.into_iter().enumerate().map(|(n,l)| html!(
                   <>
                       if n > 0 {
                           <span class="pf-c-breadcrumb__item-divider"><i class="fas fa-angle-right" aria-hidden="true"></i></span>
                       }
                       if n == last {
                           <li class="pf-c-breadcrumb__item pf-m-current">{l}</li>
                       } else {
                           <li class="pf-c-breadcrumb__item">{l}</li>
                       }
                   </>
               ))}
            </ol>
        </nav>
        </>
    )
}

fn render_type(r#type: &TypeInformation) -> Html {
    html!(
        <>
            <Content>
                <dl>
                    <dt>{"Name"}</dt>
                    <dd>
                        {r#type.name.as_deref().unwrap_or_default()}
                        if !r#type.parameters.is_empty() {
                            {"<"}
                            { for r#type.parameters.iter().map(|s|Html::from(s)) }
                            {">"}
                        }
                    </dd>
                </dl>
                <Asciidoc content={r#type.documentation.as_deref().unwrap_or_default().to_string()}/>
            </Content>
        </>
    )
}

fn render_module(base: Rc<Vec<String>>, module: &ModuleHandle) -> Html {
    let path = base.join("::");

    html!(<>
        
        <ul>
        { for module.modules.iter().map(|module| {
            let path = format!("{path}::{module}");
            html!(<li key={module.clone()}><Link<AppRoute> target={AppRoute::Repository {path}}>{&module}</Link<AppRoute>></li>)
        })}
        { for module.types.iter().map(|r#type| {
            let path = format!("{path}::{type}");
            html!(<li key={r#type.clone()}><Link<AppRoute> target={AppRoute::Repository {path}}>{&r#type}</Link<AppRoute>></li>)
        })}
        </ul>
    </>)
}
