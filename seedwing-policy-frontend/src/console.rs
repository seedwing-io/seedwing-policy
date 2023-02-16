use crate::pages::{self, AppRoute};
use patternfly_yew::*;
use yew::prelude::*;
use yew_nested_router::prelude::{Switch as RouterSwitch, *};

#[function_component(Console)]
pub fn console() -> Html {
    let logo = html! (
        <Logo src="/images/logo-inverted.png" alt="Seedwing Logo" />
    );

    let sidebar = html_nested!(
        <PageSidebar>
            <Nav>
                <NavList>
                    <NavExpandable title="Home">
                        <NavRouterItem<AppRoute> to={AppRoute::Index}>{ "Overview" }</NavRouterItem<AppRoute>>
                        <NavRouterItem<AppRoute> to={AppRoute::Documentation}>{ "Documentation" }</NavRouterItem<AppRoute>>
                        <NavRouterItem<AppRoute> to={AppRoute::Examples}>{ "Examples" }</NavRouterItem<AppRoute>>
                        <NavRouterItem<AppRoute> to={AppRoute::Playground}>{ "Playground" }</NavRouterItem<AppRoute>>
                    </NavExpandable>
                    <NavExpandable title="Repository">
                        <NavRouterItem<AppRoute> to={AppRoute::Repository{path: "".into()}}
                            predicate={AppRoute::is_repository}
                            >{ "Repository" }</NavRouterItem<AppRoute>>
                    </NavExpandable>
                </NavList>
            </Nav>
        </PageSidebar>
    );

    html!(
        <Router<AppRoute>>
            <Page {logo} {sidebar}>
                <RouterSwitch<AppRoute> {render}/>

                <PageSection variant={PageSectionVariant::Darker} fill={PageSectionFill::NoFill}>
                    {"Copyright © 2023 Red Hat, Inc. and "} <a href="https://github.com/seedwing-io" target="_blank"> {"The Seedwing Project"} </a> {"."}
                </PageSection>
            </Page>
        </Router<AppRoute>>
    )
}

fn render(route: AppRoute) -> Html {
    log::info!("Route: {route:?}");
    match route {
        AppRoute::Index => html!(<pages::Index/>),
        AppRoute::Documentation => html!(<pages::Documentation />),
        AppRoute::Repository { path } => html!(<pages::Repository {path}/>),
        AppRoute::Playground => html!(<pages::Playground />),
        _ => html!({ "Work in Progress" }),
    }
}
