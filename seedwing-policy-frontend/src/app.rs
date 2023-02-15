use crate::console::Console;
use patternfly_yew::*;
use yew::prelude::*;

#[function_component(Application)]
pub fn app() -> Html {
    html!(
        <ToastViewer>
            <BackdropViewer>
                <Console />
            </BackdropViewer>
        </ToastViewer>
    )
}
