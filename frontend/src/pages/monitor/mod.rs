use crate::pages::{AppRoute, BreadcrumbsProps};
use crate::utils::format_duration;
use anyhow::Error;
use chrono::DateTime;
use patternfly_yew::prelude::*;
use seedwing_policy_engine::lang::Severity;
use seedwing_policy_engine::runtime::monitor::{
    SimpleMonitorEvent, SimpleMonitorStart, SimpleOutput,
};
use serde_json::Value;
use yew::prelude::*;
use yew::{html, use_memo, Html};
use yew_websocket::macros::Json;
use yew_websocket::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct MonitorProps {
    pub path: AttrValue,
}

#[function_component(Monitor)]
pub fn monitor(props: &MonitorProps) -> Html {
    let parent = use_memo(
        |path| {
            let path = path.trim_start_matches(':');
            path.split("::").map(|s| s.to_string()).collect::<Vec<_>>()
        },
        props.path.clone(),
    );

    let title = html!( <Title>{"Monitor"}</Title> );
    let main = html!( <MonitorStream path={props.path.clone()}/> );

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

                path.push_str(segment);
                path.push_str("::");

                let target = AppRoute::Policy { path: path.clone() };

                html_nested!(
                    <BreadcrumbRouterItem<AppRoute>
                        to={target}
                    >
                        { if segment.is_empty() {
                            "Monitor"
                        } else {
                            segment
                        } }
                    </BreadcrumbRouterItem<AppRoute>>
                )
            })}
        </Breadcrumb>
    )
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct MonitorStreamProps {
    pub path: AttrValue,
}

#[derive(Debug)]
pub enum MonitorMessage {
    Connected,
    Data(Result<SimpleMonitorEvent, Error>),
    Lost,
}

#[derive(Eq, Clone)]
pub struct MonitorEntry {
    correlation: u64,
    name: String,
    start_timestamp: String,
    complete_timestamp: Option<String>,
    input: Value,
    output: Option<SimpleOutput>,
}

impl MonitorEntry {
    fn effective_timestamp(&self) -> &String {
        if let Some(complete) = &self.complete_timestamp {
            complete
        } else {
            &self.start_timestamp
        }
    }
}

impl PartialEq<Self> for MonitorEntry {
    fn eq(&self, other: &Self) -> bool {
        self.correlation.eq(&other.correlation)
    }
}

impl TableEntryRenderer for MonitorEntry {
    fn render_cell(&self, context: &CellContext) -> Cell {
        match context.column {
            0 => {
                html!(
                    <>
                      {&self.name} {" "}
                      <Status output={self.output.clone()}/>
                    </>
                )
            }
            1 => {
                html!(<Timing start={self.start_timestamp.clone()} complete={self.complete_timestamp.clone()}/>)
            }
            2 => {
                html!(
                    <Input input={self.input.clone()}/>
                )
            }
            3 => {
                html!(
                    <Output output={self.output.clone()}/>
                )
            }
            _ => html!(),
        }
        .into()
    }
}

#[derive(PartialEq, Properties)]
struct TimingProps {
    start: String,
    complete: Option<String>,
}

#[function_component(Timing)]
fn timing(props: &TimingProps) -> Html {
    let duration = match (
        DateTime::parse_from_rfc2822(&props.start),
        props.complete.as_deref().map(DateTime::parse_from_rfc2822),
    ) {
        (Ok(start), Some(Ok(end))) => format_duration(end - start)
            .map(|d| html!(<span>{" ("} {d} {")"}</span>))
            .unwrap_or_default(),
        _ => html!(),
    };

    html!(
        <div>
            {&props.start}
            {duration}
        </div>
    )
}

#[function_component(Status)]
fn status(props: &OutputProps) -> Html {
    if let Some(output) = &props.output {
        match output {
            SimpleOutput::Identity(severity) => {
                html!(
                    <Label label={"Identity"} color={color(*severity)} compact=true/>
                )
            }
            SimpleOutput::Transform(severity, _) => {
                html!(
                    <Label label={"Transform"} color={color(*severity)} compact=true/>
                )
            }
            SimpleOutput::Err(_) => {
                html!(
                    <Label label={"Error"} color={Color::Purple} compact=true/>
                )
            }
        }
    } else {
        html!(
          <Label label={"Running"} compact=true/>
        )
    }
}

fn color(severity: Severity) -> Color {
    match severity {
        Severity::None => Color::Green,
        Severity::Advice => Color::Cyan,
        Severity::Warning => Color::Orange,
        Severity::Error => Color::Red,
    }
}

#[derive(Clone, PartialEq, Properties)]
pub struct InputProps {
    input: Value,
}

#[function_component(Input)]
fn input(props: &InputProps) -> Html {
    if let Ok(formatted) = serde_json::to_string_pretty(&props.input) {
        html!(
            <pre><code>{formatted}</code></pre>
        )
    } else {
        html!()
    }
}

#[derive(Clone, PartialEq, Properties)]
pub struct OutputProps {
    output: Option<SimpleOutput>,
}

#[function_component(Output)]
fn output(props: &OutputProps) -> Html {
    if let Some(output) = &props.output {
        match output {
            SimpleOutput::Identity(_) => {
                html!(<i>{"Identity"}</i>)
            }
            SimpleOutput::Transform(_, value) => {
                if let Ok(formatted) = serde_json::to_string_pretty(&value) {
                    html!(
                        <pre><code>{formatted}</code></pre>
                    )
                } else {
                    html!(<i>{"error formatting value"}</i>)
                }
            }
            SimpleOutput::Err(err) => {
                html!(err)
            }
        }
    } else {
        html!(
            <i>{"in progress"}</i>
        )
    }
}

impl TryFrom<SimpleMonitorStart> for MonitorEntry {
    type Error = ();

    fn try_from(start: SimpleMonitorStart) -> Result<Self, Self::Error> {
        Ok(Self {
            correlation: start.correlation,
            name: start.name.ok_or(())?,
            start_timestamp: start.timestamp,
            complete_timestamp: None,
            input: start.input,
            output: None,
        })
    }
}

pub struct MonitorStream {
    pub ws: Option<WebSocketTask>,
    pub entries: Vec<MonitorEntry>,
}

impl MonitorStream {
    fn integrate(&mut self, event: SimpleMonitorEvent) {
        match event {
            SimpleMonitorEvent::Start(start) => {
                if let Ok(entry) = start.try_into() {
                    self.entries.push(entry);
                }
            }
            SimpleMonitorEvent::Complete(complete) => {
                if let Some(entry) = self.entries.iter_mut().find(|e| {
                    e.correlation == complete.correlation && e.complete_timestamp.is_none()
                }) {
                    entry.complete_timestamp = Some(complete.timestamp);
                    entry.output = Some(complete.output);
                }
            }
        }
    }
}

impl Component for MonitorStream {
    type Message = MonitorMessage;
    type Properties = MonitorStreamProps;

    fn create(ctx: &Context<Self>) -> Self {
        let callback = ctx
            .link()
            .callback(|Json(snapshot)| MonitorMessage::Data(snapshot));
        let notification = ctx.link().batch_callback(|status| match status {
            WebSocketStatus::Opened => Some(MonitorMessage::Connected),
            WebSocketStatus::Closed => Some(MonitorMessage::Lost),
            WebSocketStatus::Error => Some(MonitorMessage::Lost),
        });
        let task = WebSocketService::connect_text(
            //todo figure out why trunk ws proxy isn't --> format!("ws://localhost:8010/stream/statistics/v1alpha1/").as_str(),
            format!(
                "ws://localhost:8080/stream/monitor/v1alpha1/{}",
                ctx.props().path
            )
            .as_str(),
            callback,
            notification,
        )
        .unwrap();

        Self {
            ws: Some(task),
            entries: vec![],
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            MonitorMessage::Data(response) => {
                if let Ok(event) = response {
                    self.integrate(event);
                }
            }
            MonitorMessage::Lost => {
                self.ws = None;
            }
            MonitorMessage::Connected => {}
        }

        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let header = html_nested! {
            <TableHeader>
                <TableColumn label="Pattern" width={ColumnWidth::Percent(20)}/>
                <TableColumn label="Timing" width={ColumnWidth::Percent(20)}/>
                <TableColumn label="Input" width={ColumnWidth::Percent(30)}/>
                <TableColumn label="Output" width={ColumnWidth::Percent(30)}/>
            </TableHeader>
        };

        let mut rows = self.entries.clone();

        rows.sort_by(|l, r| r.effective_timestamp().cmp(l.effective_timestamp()));
        let entries = SharedTableModel::new(rows);

        html!(
            <Table<SharedTableModel<MonitorEntry>> {header} {entries} mode={TableMode::Compact}/>
        )
    }
}
