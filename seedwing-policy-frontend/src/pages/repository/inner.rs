use patternfly_yew::prelude::*;
use seedwing_policy_engine::api::TypeInformation;
use std::rc::Rc;
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct Props {
    pub r#type: Rc<TypeInformation>,
}

#[function_component(Inner)]
pub fn inner(props: &Props) -> Html {
    html!(
        <>
            <CodeBlock>
            </CodeBlock>
        </>
    )
}
