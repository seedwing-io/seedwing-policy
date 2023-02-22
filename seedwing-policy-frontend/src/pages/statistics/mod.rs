use crate::pages::BreadcrumbsProps;
use crate::{
    common::{
        editor::Editor,
        eval::{validate, ResultView},
    },
    pages::AppRoute,
};
use gloo_net::http::Request;
use patternfly_yew::*;
use seedwing_policy_engine::api::ComponentInformation;
use seedwing_policy_engine::runtime::statistics::Snapshot;
use yew::prelude::*;
use yew::{html, use_effect_with_deps, use_memo, AttrValue, Html};
use yew_hooks::{use_async, UseAsyncState};

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct Props {
    pub path: AttrValue,
}

pub async fn fetch(path: &Vec<String>) -> Result<Option<Vec<Snapshot>>, String> {
    log::info!("fetching: {path:?}");

    // FIXME: urlencode segments
    let path = path.join("/");

    let response = Request::get(&format!("/api/statistics/v1alpha1/{}", path))
        .header("Accept", "application/json")
        .send()
        .await;

    println!("{:?}", response);
    match response {
        Ok(response) => {
            println!("no direct error");
            if response.status() == 404 {
                Ok(None)
            } else {
                Ok(Some(response.json().await.map_err(|err| err.to_string())?))
            }
        }
        Err(e) => {
            println!("got an error {}", e);
            Err(e.to_string())
        }
    }
}

#[function_component(Statistics)]
pub fn statistics(props: &Props) -> Html {
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

    let main = match &*state {
        UseAsyncState { loading: true, .. } => html!({ "Loading..." }),
        UseAsyncState {
            loading: false,
            error: Some(error),
            ..
        } => html!(<> {"Failed: "} {error} </>),

        UseAsyncState {
            data: Some(Some(snapshots)),
            ..
        } => (html!(<Snapshots snapshots={snapshots.clone()}/>)),
        _ => html!("Unknown state"),
    };

    let title = html!( <Title>{"Statistics"}</Title> );

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

#[function_component(Breadcrumbs)]
fn render_breadcrumbs(props: &BreadcrumbsProps) -> Html {
    let mut path = String::new();

    let root = vec![String::new()];
    let bpath = root.iter().chain(props.parent.iter());

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
                            "Statistics"
                        } else {
                            &segment
                        } }
                    </BreadcrumbRouterItem<AppRoute>>
                )
            })}
        </Breadcrumb>
    )
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

#[derive(Clone, PartialEq, Properties)]
struct StatisticsProps {
    snapshots: Vec<Snapshot>,
}

#[derive(PartialEq, Clone)]
struct RenderableSnapshot(Snapshot);

impl TableRenderer for RenderableSnapshot {
    fn render(&self, column: ColumnIndex) -> Html {
        match column.index {
            0 => html!(&self.0.name),
            1 => html!(&self.0.invocations),
            2 => html!(&self.0.satisfied_invocations),
            3 => html!(&self.0.unsatisfied_invocations),
            4 => html!(&self.0.error_invocations),
            5 => html!(&self.0.mean),
            6 => html!(&self.0.median),
            7 => html!(&self.0.stddev),
            _ => html!(),
        }
    }
}

#[function_component(Snapshots)]
fn snapshots(props: &StatisticsProps) -> Html {
    let header = html_nested! {
        <TableHeader>
            <TableColumn label="Pattern"/>
            <TableColumn label="Total"/>
            <TableColumn label="Satisfied"/>
            <TableColumn label="Unsatisfied"/>
            <TableColumn label="Error"/>
            <TableColumn label="Mean"/>
            <TableColumn label="Median"/>
            <TableColumn label="StdDev"/>
        </TableHeader>
    };
    let snapshots = props
        .snapshots
        .iter()
        .map(|e| RenderableSnapshot(e.clone()))
        .collect();
    let entries = SharedTableModel::new(snapshots);
    html!(
        <Table<SharedTableModel<RenderableSnapshot>> {header} {entries}/>
    )
}
