use monaco::{
    api::{CodeEditorOptions, DisposableClosure, TextModel},
    sys::editor::{BuiltinTheme, IModelContentChangedEvent, IStandaloneEditorConstructionOptions},
    yew::{CodeEditor, CodeEditorLink},
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
    #[prop_or_default]
    pub on_editor_created: Callback<CodeEditorLink>,
    #[prop_or(true)]
    pub auto_layout: bool,
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
            .with_automatic_layout(ctx.props().auto_layout)
            .to_sys_options();

        Self {
            model,
            options,
            _listener: listener,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().initial_content != old_props.initial_content {
            self.model.set_value(&ctx.props().initial_content);
        }

        monaco::sys::editor::set_model_markers(
            self.model.as_ref().as_ref(),
            "dogma",
            &MarkerData::array(&ctx.props().markers),
        );

        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let classes = classes!("monaco-wrapper");

        html!(
            <CodeEditor
                {classes}
                model={self.model.clone()}
                options={self.options.clone()}
                on_editor_created={ctx.props().on_editor_created.clone()}
            />
        )
    }
}
