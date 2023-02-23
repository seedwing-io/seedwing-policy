use std::cmp::Ordering;
use crate::pages::AppRoute;
use crate::pages::BreadcrumbsProps;
use anyhow::Error;
use patternfly_yew::*;
use serde_json::Value;
use yew::prelude::*;
use yew::{html, use_memo, Html};
use yew_websocket::macros::Json;
use yew_websocket::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use seedwing_policy_engine::runtime::monitor::{SimpleMonitorEvent, SimpleMonitorStart, SimpleOutput};

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct MonitorProps {
    pub path: AttrValue,
}

#[function_component(Monitor)]
pub fn monitor(props: &MonitorProps) -> Html {
    let parent = use_memo(
        |path| {
            let path = path.trim_start_matches(":");
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

                path.push_str(&segment);
                path.push_str("::");

                let target = AppRoute::Policy { path: path.clone() };

                html_nested!(
                    <BreadcrumbRouterItem<AppRoute>
                        to={target}
                    >
                        { if segment.is_empty() {
                            "Monitor"
                        } else {
                            &segment
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

impl TableRenderer for MonitorEntry {
    fn render(&self, column: ColumnIndex) -> Html {
        match column.index {
            0 => {
                html!(
                    <>
                      <div>
                        {&self.name}
                      </div>
                      <Status output={self.output.clone()}/>
                    </>
                )
            },
            1 => if let Some(ts) = &self.complete_timestamp {
                html!(
                    <>
                      <div>{&self.start_timestamp}</div>
                      <div>{ts}</div>
                    </>
                )
            } else {
                html!(<div>{&self.start_timestamp}</div>)
            },
            2 => {
                html!(
                    <InputOutput input={self.input.clone()} output={self.output.clone()}/>
                )
            }
            _ => html!(),
        }
    }
}

#[derive(Clone, PartialEq, Properties)]
pub struct InputOutputProps {
    input: Value,
    output: Option<SimpleOutput>,
}

#[function_component(InputOutput)]
fn input_output(props: &InputOutputProps) -> Html {
    let input_formatted = serde_json::to_string_pretty(&props.input);
    html!(
        <Tabs>
          <Tab label="Input">
            <Input input={props.input.clone()}/>
          </Tab>
          <Tab label="Output">
            <Output output={props.output.clone()}/>
          </Tab>
        </Tabs>
    )
}

#[function_component(Status)]
fn status(props: &OutputProps) -> Html {
    if let Some(output) = &props.output {
        match output {
            SimpleOutput::None => {
                html!(
                    <Label label={"Unsatisfied"} color={Color::Red}/>
                )
            }
            SimpleOutput::Identity => {
                html!(
                    <Label label={"Identity"} color={Color::Green}/>
                )
            }
            SimpleOutput::Transform(_) => {
                html!(
                    <Label label={"Transform"} color={Color::Green}/>
                )
            }
            SimpleOutput::Err(_) => {
                html!(
                    <Label label={"Error"} color={Color::Purple}/>
                )
            }
        }
    } else {
        html!(
          <Label label={"Running"}/>
        )
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
            <pre><code>
                {formatted}
            </code></pre>
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
            SimpleOutput::None => {
                html!(<i>{"None"}</i>)
            }
            SimpleOutput::Identity => {
                html!(<i>{"Identity"}</i>)
            }
            SimpleOutput::Transform(value) => {
                if let Ok(formatted) = serde_json::to_string_pretty(&props.output) {
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
                if let Some(entry) = self.entries.iter_mut().find(|e| e.correlation == complete.correlation && e.complete_timestamp.is_none()) {
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
            format!("ws://localhost:8080/stream/monitor/v1alpha1/{}", ctx.props().path).as_str(),
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
                <TableColumn label="Pattern"/>
                <TableColumn label="Timing"/>
                <TableColumn label="Input/Output"/>
            </TableHeader>
        };

        let mut rows = self.entries.clone();

        rows.sort_by( |l, r| r.effective_timestamp().cmp( l.effective_timestamp()));
        let entries = SharedTableModel::new(rows );

        html!(
            <Table<SharedTableModel<MonitorEntry>> {header} {entries} mode={TableMode::Compact}/>
        )
    }
}

