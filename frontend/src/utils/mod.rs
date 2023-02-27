use chrono::Duration;
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

pub fn format_duration(duration: Duration) -> Option<String> {
    match duration.num_nanoseconds() {
        Some(ns) if ns < 0 => {
            let mut r = format_ns((-ns) as u128);
            r.insert(0, '-');
            Some(r)
        }
        Some(ns) => Some(format_ns(ns as u128)),
        None => None,
    }
}

pub fn format_ns(ns: u128) -> String {
    let ms = ns / 1_000_000;
    let ns = ns - (ms * 1_000_000);

    let sec = ms / 1_000;
    let ms = ms - (sec * 1_000);

    if sec > 0 {
        format!("{}s {}ms", sec, ms)
    } else if ms > 0 {
        format!("{}ms", ms)
    } else {
        format!("{}ns", ns)
    }
}
