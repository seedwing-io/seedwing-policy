use monaco::{
    api::{CodeEditorOptions, DisposableClosure, TextModel},
    sys::editor::{BuiltinTheme, IModelContentChangedEvent, IStandaloneEditorConstructionOptions},
    yew::CodeEditor,
};
use yew::prelude::*;

mod marker;
pub use marker::*;

#[derive(PartialEq, Properties)]
pub struct EditorProps {
    #[prop_or_default]
    pub initial_content: String,
    pub language: String,
    #[prop_or_default]
    pub on_change: Callback<String>,
    #[prop_or_default]
    pub markers: Vec<MarkerData>,
}

pub struct Editor {
    model: TextModel,
    options: IStandaloneEditorConstructionOptions,
    _listener: DisposableClosure<dyn FnMut(IModelContentChangedEvent)>,
}

impl Component for Editor {
    type Message = ();
    type Properties = EditorProps;

    fn create(ctx: &Context<Self>) -> Self {
        let model = TextModel::create(
            &ctx.props().initial_content,
            Some(&ctx.props().language),
            None,
        )
        .unwrap();

        let listener = {
            let cb = {
                let model = model.clone();
                let on_change = ctx.props().on_change.clone();

                move |_| {
                    let content = model.get_value();
                    on_change.emit(content);
                }
            };
            model.on_did_change_content(cb)
        };

        // emit the initial data once

        ctx.props()
            .on_change
            .emit(ctx.props().initial_content.clone());

        // prepare editor options

        let options = CodeEditorOptions::default()
            .with_scroll_beyond_last_line(false)
            .with_language(ctx.props().language.clone())
            .with_builtin_theme(BuiltinTheme::Vs)
            .with_automatic_layout(true)
            .to_sys_options();

        Self {
            model,
            options,
            _listener: listener,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        monaco::sys::editor::set_model_markers(
            self.model.as_ref().as_ref(),
            "dogma",
            &MarkerData::array(&ctx.props().markers),
        );

        false
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let classes = classes!("monaco-wrapper");

        html!(
            <CodeEditor
                {classes}
                model={self.model.clone()}
                options={self.options.clone()}
            />
        )
    }
}
