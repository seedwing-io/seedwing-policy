use crate::pages::AppRoute;
use patternfly_yew::*;
use yew::prelude::*;
use yew_nested_router::prelude::use_router;

#[function_component(Index)]
pub fn index() -> Html {
    let router = use_router::<AppRoute>();

    let primary = Callback::from(move |_| {
        if let Some(router) = &router {
            router.push(AppRoute::Playground);
        }
    })
    .into_action("Playground");

    let secondaries = vec![
        Callback::from(|_| {
            let _ = gloo_utils::window().open_with_url_and_target(
                "https://github.com/seedwing-io/seedwing-policy",
                "_blank",
            );
        })
        .into_action("GitHub"),
        Callback::from(|_| {
            let _ = gloo_utils::window().open_with_url_and_target(
                "https://raw.githubusercontent.com/seedwing-io/seedwing-policy/main/LICENSE",
                "_blank",
            );
        })
        .into_action("License"),
    ];

    html!(
        <>
            <PageSection variant={PageSectionVariant::Light} fill=true>
                <Bullseye>
                    <EmptyState
                        full_height=true
                        title="Seedwing Policy Engine"
                        icon={Icon::Catalog}
                        {primary}
                        {secondaries}
                    >
                        { "If it looks like a duck, swims like a duck, and quacks like a duck, then you better enforce with a policy that it indeed is a duck!"}
                    </EmptyState>
                </Bullseye>
            </PageSection>
        </>
    )
}
