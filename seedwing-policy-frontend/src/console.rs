use crate::{
    pages::{self, AppRoute},
    utils::{use_open, ExtLinkIcon},
};
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
                        <NavRouterItem<AppRoute> to={AppRoute::Playground}>{ "Playground" }</NavRouterItem<AppRoute>>
                        <NavItem to="https://github.com/seedwing-io/seedwing-policy" target="_blank">{ "Documentation" } <ExtLinkIcon/> </NavItem>
                        <NavItem to="https://github.com/seedwing-io/seedwing-policy" target="_blank">{ "Examples" } <ExtLinkIcon/> </NavItem>
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

    let callback_help = use_open("https://github.com/seedwing-io/seedwing-policy", "_blank");
    let callback_github = use_open("https://github.com/seedwing-io/seedwing-policy", "_blank");

    let tools = html!(
        <Toolbar>
            <ToolbarItem>
                <Button icon={Icon::QuestionCircle} onclick={callback_help}/>
            </ToolbarItem>
            <ToolbarItem>
                <Button icon={Icon::Github} onclick={callback_github}/>
            </ToolbarItem>
        </Toolbar>
    );

    html!(
        <Router<AppRoute>>
            <Page {logo} {sidebar} {tools}>
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
        AppRoute::Repository { path } => html!(<pages::Repository {path}/>),
        AppRoute::Playground => html!(<pages::Playground />),
    }
}
