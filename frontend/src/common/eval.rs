use patternfly_yew::prelude::*;
use seedwing_policy_engine::runtime::response::{Name, Response};
use serde_json::Value;
use std::rc::Rc;
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct ResultViewProps {
    pub result: Response,
}

#[derive(Clone, PartialEq)]
pub struct ResponseModel(Response);

impl TreeTableModel for ResponseModel {
    fn children(&self) -> Vec<Rc<dyn TreeNode>> {
        vec![Rc::new(self.clone()) as Rc<dyn TreeNode>]
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
                let state = match self.0.satisfied {
                    true => HelperTextState::Success,
                    false => HelperTextState::Error,
                };
                html!(
                    <>
                        <HelperText>
                            <HelperTextItem {state}>{ &self.0.reason }</HelperTextItem>
                        </HelperText>
                    </>
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

#[function_component(ResultTreeView)]
pub fn result_tree(props: &ResultViewProps) -> Html {
    let header = html_nested! {
        <TreeTableHeader>
            <TableColumn label="Name" width={ColumnWidth::FitContent} />
            <TableColumn label="Result" width={ColumnWidth::Percent(20)} />
            <TableColumn label="Input / Output"/>
        </TreeTableHeader>
    };

    html!(<TreeTable<ResponseModel> mode={TreeTableMode::Compact} {header} model={Rc::new(ResponseModel(props.result.clone()))}/>)
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

    let response = gloo_net::http::Request::post(&format!("/api/playground/v1alpha1/evaluate"))
        .query([("format", "json")])
        .json(&request)
        .map_err(|err| format!("Failed to encode request: {err}"))?
        .send()
        .await
        .map_err(|err| format!("Failed to send eval request: {err}"))?;

    response
        .json::<Response>()
        .await
        .map_err(|err| format!("Failed to read response: {err}"))
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
