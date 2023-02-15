use yew::prelude::*;

#[function_component(Documentation)]
pub fn docs() -> Html {
    html!(<iframe src="/docs"/>)
}
