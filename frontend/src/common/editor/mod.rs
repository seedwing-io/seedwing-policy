use monaco::{
    api::{CodeEditorOptions, DisposableClosure, TextModel},
    sys::editor::{BuiltinTheme, IModelContentChangedEvent, IStandaloneEditorConstructionOptions},
    yew::{CodeEditor, CodeEditorLink},
};
use std::marker::PhantomData;
use std::ops::Deref;
use uuid::Uuid;
use yew::prelude::*;

mod marker;

pub use marker::*;

pub trait InitialContent: AsRef<str> + Default + PartialEq {}

impl InitialContent for String {}

#[derive(Default, PartialEq)]
pub struct Generation<I>(I, Uuid);

impl<I> Generation<I> {
    pub fn new(value: I) -> Self {
        Self(value, Uuid::new_v4())
    }

    pub fn as_ref(&self) -> Generation<&I> {
        Generation(&self.0, self.1)
    }

    pub fn map<T, F>(self, f: F) -> Generation<T>
    where
        F: FnOnce(I) -> T,
    {
        Generation(f(self.0), self.1)
    }
}

impl<I> AsRef<str> for Generation<I>
where
    I: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<I> InitialContent for Generation<I> where I: InitialContent {}

impl<I> From<I> for Generation<I> {
    fn from(value: I) -> Self {
        Self::new(value)
    }
}

impl<T> Deref for Generation<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(PartialEq, Properties)]
pub struct EditorProps<I: InitialContent = String> {
    #[prop_or_default]
    pub initial_content: I,
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

pub struct Editor<I: InitialContent = String> {
    model: TextModel,
    options: IStandaloneEditorConstructionOptions,
    _listener: DisposableClosure<dyn FnMut(IModelContentChangedEvent)>,
    _marker: PhantomData<I>,
}

impl<I: InitialContent> Component for Editor<I>
where
    I: 'static,
{
    type Message = ();
    type Properties = EditorProps<I>;

    fn create(ctx: &Context<Self>) -> Self {
        let model = TextModel::create(
            ctx.props().initial_content.as_ref(),
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
            .emit(ctx.props().initial_content.as_ref().to_string());

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
            _marker: Default::default(),
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().initial_content != old_props.initial_content {
            self.model.set_value(ctx.props().initial_content.as_ref());
        }

        monaco::sys::editor::set_model_markers(
            self.model.as_ref().as_ref(),
            &ctx.props().language,
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
