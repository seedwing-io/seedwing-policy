use patternfly_yew::prelude::*;
use seedwing_policy_engine::{
    lang::Severity,
    runtime::response::{Collector, Name, Response},
};
use serde_json::Value;
use std::rc::Rc;
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct ResultViewProps {
    pub result: Vec<Response>,
}

#[derive(Clone, PartialEq)]
pub struct RationaleModel(Vec<Response>);

#[derive(Clone, PartialEq)]
pub struct ResponseModel(Response);

impl TreeTableModel for RationaleModel {
    fn children(&self) -> Vec<Rc<dyn TreeNode>> {
        // vec![Rc::new(self.clone()) as Rc<dyn TreeNode>]
        self.0
            .iter()
            .map(|r| Rc::new(ResponseModel(r.clone())) as Rc<dyn TreeNode>)
            .collect::<Vec<_>>()
    }
}

impl TreeNode for ResponseModel {
    fn render_main(&self) -> Cell {
        let name = self.0.name.clone();
        html!(
            if let Name::Pattern(None) = name {
                <em>{"Unnamed"}</em>
            } else {
                <PatternNameView {name} />
            }
        )
        .into()
    }

    fn render_cell(&self, ctx: CellContext) -> Cell {
        match ctx.column {
            0 => {
                let (icon,state) = match self.0.severity {
                    Severity::None => (None, HelperTextState::Success),
                    Severity::Advice=> (Some(Icon::InfoCircle), HelperTextState::Default),
                    Severity::Warning => (None, HelperTextState::Warning),
                    Severity::Error => (None, HelperTextState::Error),
                };
                html!(
                    <HelperText>
                        <HelperTextItem {state} {icon}>{ &self.0.reason }</HelperTextItem>
                    </HelperText>
                )
            }
            1 => {
                let input = serde_json::to_string_pretty(&self.0.input).unwrap_or_default();
                let (output, show) = self.0.output.as_ref().and_then(|output|serde_json::to_string_pretty(output).ok()).map(|output|{
                    let show = output != input;
                    (output, show)
                }).unzip();

                let show = show.unwrap_or_default();
                html!(
                    <>
                        <Clipboard code=true readonly=true variant={ClipboardVariant::Expandable} value={input}/>
                        if show {
                            if let Some(output) = output {
                                <Clipboard code=true readonly=true variant={ClipboardVariant::Expandable} value={output}/>
                            }
                        }
                    </>
                )
            }
            _ => Html::default(),
        }
        .into()
    }

    fn children(&self) -> Vec<Rc<dyn TreeNode>> {
        self.0
            .rationale
            .iter()
            .map(|r| Rc::new(ResponseModel(r.clone())) as Rc<dyn TreeNode>)
            .collect::<Vec<_>>()
    }
}

#[function_component(ResultView)]
pub fn result(props: &ResultViewProps) -> Html {
    html!(
        <div>
            <Tabs>
                <Tab label="Response">
                    <ResultTreeView result={props.result.clone()}/>
                </Tab>
                <Tab label="Compact">
                    <CompactView result={props.result.clone()}/>
                </Tab>
                <Tab label="JSON">
                    {
                        match serde_json::to_string_pretty(&props.result) {
                            Ok(json) => {
                                html!(<CodeBlock><CodeBlockCode>{ json }</CodeBlockCode></CodeBlock>)
                            }
                            Err(err) => format!("Failed to render as JSON: {err}").into(),
                        }
                    }

                </Tab>
            </Tabs>
        </div>
    )
}

#[function_component(CompactView)]
pub fn compact(props: &ResultViewProps) -> Html {
    let severity = use_state_eq(|| Severity::Error);

    let compact = use_memo(
        |(response, severity)| {
            response
                .iter()
                .flat_map(|r| Collector::new(r).with_severity(**severity).collect())
                .collect::<Vec<_>>()
        },
        (props.result.clone(), severity.clone()),
    );

    let onselect = {
        let severity = severity.clone();
        Callback::from(move |item| {
            severity.set(item);
        })
    };

    html!(
        <>
            <Select<Severity> variant={SelectVariant::Single(onselect)} initial_selection={vec![*severity]}>
                <SelectOption<Severity> value={Severity::Advice} />
                <SelectOption<Severity> value={Severity::Warning} />
                <SelectOption<Severity> value={Severity::Error} />
            </Select<Severity>>
            <ResultTreeView result={(*compact).clone()} />
        </>
    )
}

#[function_component(ResultTreeView)]
pub fn result_tree(props: &ResultViewProps) -> Html {
    let header = html_nested! {
        <TreeTableHeader>
            <TableColumn label="Name" width={ColumnWidth::FitContent} />
            <TableColumn label="Result" width={ColumnWidth::Percent(20)} />
            <TableColumn label="Input / Output"/>
        </TreeTableHeader>
    };

    html!(<TreeTable<RationaleModel> mode={TreeTableMode::Compact} {header} model={Rc::new(RationaleModel(props.result.clone()))}/>)
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
struct PatternNameProps {
    pub name: Name,
}

#[function_component(PatternNameView)]
fn pattern_name(props: &PatternNameProps) -> Html {
    html!(
        <span>
            { props.name.to_string() }
        </span>
    )
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct EvaluateRequest {
    name: String,
    policy: String,
    value: Value,
}

/// Do a remote evaluation with the server
pub async fn eval(policy: String, name: String, value: Value) -> Result<Response, String> {
    let request = EvaluateRequest {
        name,
        policy,
        value,
    };

    let response = gloo_net::http::Request::post("/api/playground/v1alpha1/evaluate")
        .query([("format", "json"), ("no_error", "true")])
        .json(&request)
        .map_err(|err| format!("Failed to encode request: {err}"))?
        .send()
        .await
        .map_err(|err| format!("Failed to send eval request: {err}"))?;

    match response.ok() {
        true => response
            .json::<Response>()
            .await
            .map_err(|err| format!("Failed to read response: {err}")),
        false => {
            let cause = response
                .text()
                .await
                .map_err(|err| format!("Failed to read error response: {err}"))?;
            Err(match cause.is_empty() {
                true => format!("{} ({})", response.status_text(), response.status()),
                false => cause,
            })
        }
    }
}

/// Validate a remote policy
pub async fn validate(path: &str, value: Value) -> Result<Response, String> {
    let response = gloo_net::http::Request::post(&format!("/api/policy/v1alpha1/{path}"))
        .query([("format", "json")])
        .json(&value)
        .map_err(|err| format!("Failed to encode request: {err}"))?
        .send()
        .await
        .map_err(|err| format!("Failed to send request: {err}"))?;

    match (response.ok(), response.status()) {
        (true, _) | (false, 406 | 422) => response
            .json::<Response>()
            .await
            .map_err(|err| format!("Failed to read response: {err}")),
        (false, _) => {
            let payload = response
                .text()
                .await
                .map_err(|err| format!("Failed to read error response: {err}"))?;
            Err(format!(
                "{} {}: {payload}",
                response.status(),
                response.status_text()
            ))
        }
    }
}
