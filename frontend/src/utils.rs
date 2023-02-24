use patternfly_yew::prelude::*;
use yew::prelude::*;

#[hook]
pub fn use_open<IN>(url: impl Into<String>, target: impl Into<String>) -> Callback<IN, ()>
where
    IN: 'static,
{
    use_callback(
        |_, (url, target)| {
            let _ = gloo_utils::window().open_with_url_and_target(&url, &target);
        },
        (url.into(), target.into()),
    )
}

#[function_component(ExtLinkIcon)]
pub fn ext_link_icon() -> Html {
    html!(<span class="pf-u-icon-color-light pf-u-ml-sm pf-u-font-size-sm">{ Icon::ExternalLinkAlt }</span>)
}
