pub mod sources;

use crate::core::{Function, FunctionError};
use crate::lang::hir;
use crate::lang::hir::MemberQualifier;
use crate::lang::lir;
use crate::lang::lir::{Bindings, Field, ObjectType};
use crate::lang::mir::TypeHandle;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::{
    CompilationUnit, Located, ParserError, ParserInput, PolicyParser, SourceLocation, SourceSpan,
};
use crate::lang::PackagePath;
use crate::lang::TypeName;
use crate::package::Package;
use crate::runtime::cache::SourceCache;
use crate::value::Value;
use ariadne::Cache;
use async_mutex::Mutex;
use chumsky::{Error, Stream};
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::future::{ready, Future};
use std::mem;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::ready;

pub mod cache;

pub enum Component {
    Module(ModuleHandle),
    Type(Arc<TypeHandle>),
}

#[derive(Clone, Debug)]
pub enum BuildError {
    TypeNotFound(SourceLocation, SourceSpan, String),
    Parser(SourceLocation, ParserError),
}

impl BuildError {
    pub fn source_location(&self) -> SourceLocation {
        match self {
            BuildError::TypeNotFound(loc, _, _) => loc.clone(),
            BuildError::Parser(loc, _) => loc.clone(),
        }
    }

    pub fn span(&self) -> SourceSpan {
        match self {
            BuildError::TypeNotFound(_, span, _) => span.clone(),
            BuildError::Parser(_, err) => err.span(),
        }
    }
}

impl From<(SourceLocation, ParserError)> for BuildError {
    fn from(inner: (SourceLocation, ParserError)) -> Self {
        Self::Parser(inner.0, inner.1)
    }
}

pub type EvaluationResult = Option<Arc<Mutex<Value>>>;

//#[derive(Default, Debug)]
//pub struct EvaluationResult {
//value: Option<Arc<Mutex<Value>>>,
//}

/*
impl EvaluationResult {
    pub fn new() -> Self {
        Self { value: None }
    }

    pub fn set_value(mut self, value: Arc<Mutex<Value>>) -> Self {
        self.value.replace(value);
        self
    }

    pub fn value(&self) -> &Option<Arc<Mutex<Value>>> {
        &self.value
    }

    pub fn matches(&self) -> bool {
        self.value.is_some()
    }
}

 */

#[derive(Debug)]
pub enum RuntimeError {
    Lock,
    InvalidState,
    NoSuchType(TypeName),
    Function(FunctionError),
}

#[derive(Debug)]
pub struct ModuleHandle {
    modules: Vec<String>,
    types: Vec<String>,
}

impl ModuleHandle {
    fn new() -> Self {
        Self {
            modules: vec![],
            types: vec![],
        }
    }

    fn sort(mut self) -> Self {
        self.modules.sort();
        self.types.sort();
        self
    }

    fn is_empty(&self) -> bool {
        self.modules.is_empty() && self.types.is_empty()
    }

    pub async fn to_html(&self) -> String {
        let mut html = String::new();

        html.push_str("<div>");
        if !self.modules.is_empty() {
            html.push_str("<h1>modules</h1>");

            html.push_str("<ul>");
            for module in &self.modules {
                html.push_str("<li>");
                html.push_str(format!("<a href='{}/'>{}</a>", module, module).as_str());
                html.push_str("</li>");
            }
            html.push_str("</ul>");
        }

        if !self.types.is_empty() {
            html.push_str("<h1>types</h1>");
            html.push_str("<ul>");
            for ty in &self.types {
                html.push_str("<li>");
                html.push_str(format!("<a href='{}'>{}</a>", ty, ty).as_str());
                html.push_str("</li>");
            }
            html.push_str("</ul>");
        }

        html.push_str("</div>");

        html
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::{Directory, Ephemeral};
    use serde_json::json;
    use std::default::Default;
    use std::env;
    use std::iter::once;

    #[actix_rt::test]
    async fn ephemeral_sources() {
        let src = Ephemeral::new("foo::bar", "type bob");

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let result = builder.finish().await;

        assert!(matches!(result, Ok(_)));
    }

    #[actix_rt::test]
    async fn link_test_data() {
        let src = Directory::new(env::current_dir().unwrap().join("test-data"));

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let result = builder.finish().await;

        assert!(matches!(result, Ok(_)));
    }

    #[actix_rt::test]
    async fn evaluate_function() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
            # Single-line comment, yay
            type signed-thing = {
                digest: sigstore::SHA256(
                    n<1>::{
                        apiVersion: "0.0.1",
                        spec: {
                            signature: {
                                publicKey: {
                                    content: base64::Base64(
                                        x509::PEM( n<1>::{
                                            version: 2,
                                            extensions: n<1>::{
                                                subjectAlternativeName: n<1>::{
                                                    rfc822: "bob@mcwhirter.org",
                                                }
                                            }
                                        } )
                                    )
                                }
                            }
                        }
                    }
                )
            }
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        let value = json!(
            {
                "digest": "5dd1e2b50b89874fd086da4b61176167ae9e4b434945325326690c8f604d0408"
            }
        );

        let result = runtime.evaluate("foo::bar::signed-thing", value).await;

        assert!(matches!(result, Ok(Some(_)),))
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_literals() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        type named<name> = {
            name: name
        }

        type jim = named<"Jim">
        type bob = named<"Bob">

        type folks = jim || bob

        "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        let good_bob = json!(
            {
                "name": "Bob",
                "age": 52,
            }
        );

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Bob",
                            "age": 52,
                        }
                    )
                )
                .await,
            Ok(Some(_))
        ));
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_types() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
                type named<name> = {
                    name: name
                }

                type jim = named<integer>
                type bob = named<"Bob">

                type folks = jim || bob

                "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Bob",
                            "age": 52,
                        }
                    )
                )
                .await,
            Ok(Some(_))
        ));
    }

    #[actix_rt::test]
    async fn evaluate_matches() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        type bob = {
            name: "Bob",
            age: $(self > 48),
        }

        type jim = {
            name: "Jim",
            age: $(self > 52),
        }

        type folks = bob || jim

        "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Bob",
                            "age": 49,
                        }
                    )
                )
                .await,
            Ok(Some(_))
        ));

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Jim",
                            "age": 49,
                        }
                    )
                )
                .await,
            Ok(None)
        ));

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Bob",
                            "age": 42,
                        }
                    )
                )
                .await,
            Ok(None)
        ));

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Jim",
                            "age": 53,
                        }
                    )
                )
                .await,
            Ok(Some(_))
        ));
    }
}
