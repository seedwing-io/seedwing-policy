use crate::console::Console;
use patternfly_yew::prelude::*;
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
