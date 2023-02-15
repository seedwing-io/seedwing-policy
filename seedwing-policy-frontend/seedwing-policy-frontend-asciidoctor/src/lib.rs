use yew::prelude::*;

mod sys;

pub fn convert(content: &str, options: &serde_json::Value) -> String {
    sys::Asciidoctor::convert(content, serde_wasm_bindgen::to_value(options).unwrap())
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct Props {
    #[prop_or_default]
    pub content: Option<String>,
    #[prop_or_default]
    pub options: serde_json::Value,
}

#[function_component(Asciidoc)]
pub fn asciidoc(props: &Props) -> Html {
    html!(
        <div class="asciidoctor">
            {Html::from_html_unchecked(
               convert(props.content.as_deref().unwrap_or_default(), &props.options).into(),
            )}
        </div>
    )
}
