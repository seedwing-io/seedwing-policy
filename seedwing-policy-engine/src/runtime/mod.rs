pub mod sources;

use crate::core::Function;
use crate::lang::hir;
use crate::lang::hir::MemberQualifier;
use crate::lang::lir;
use crate::lang::lir::{Bindings, Field, ObjectType, Type};
use crate::lang::mir::TypeHandle;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::{
    CompilationUnit, Located, ParserError, ParserInput, PolicyParser, SourceLocation, SourceSpan,
};
use crate::lang::PackagePath;
use crate::lang::TypeName;
use crate::package::Package;
use crate::runtime::cache::SourceCache;
use crate::runtime::rationale::Rationale;
use crate::value::RuntimeValue;
use ariadne::Cache;
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
pub mod rationale;

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

#[derive(Debug, Clone)]
pub enum Output {
    None,
    Identity,
    Transform(Rc<RuntimeValue>),
}

impl Output {
    pub fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone)]
pub struct EvaluationResult {
    input: Option<Rc<RuntimeValue>>,
    ty: Arc<Type>,
    rationale: Rationale,
    output: Output,
}

impl EvaluationResult {
    pub fn new(
        input: Option<Rc<RuntimeValue>>,
        ty: Arc<Type>,
        rationale: Rationale,
        output: Output,
    ) -> Self {
        Self {
            input,
            ty,
            rationale,
            output,
        }
    }

    pub fn satisfied(&self) -> bool {
        self.rationale.satisfied()
    }

    pub fn ty(&self) -> Arc<Type> {
        self.ty.clone()
    }

    pub fn input(&self) -> Option<Rc<RuntimeValue>> {
        self.input.clone()
    }

    pub fn rationale(&self) -> &Rationale {
        &self.rationale
    }

    pub fn output(&self) -> Option<Rc<RuntimeValue>> {
        match &self.output {
            Output::None => None,
            Output::Identity => self.input.clone(),
            Output::Transform(inner) => Some(inner.clone()),
        }
    }

    pub fn raw_output(&self) -> &Output {
        &self.output
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    Lock,
    InvalidState,
    NoSuchType(TypeName),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::{Directory, Ephemeral};
    use crate::value::RationaleResult;
    use serde_json::json;
    use std::default::Default;
    use std::env;
    use std::iter::once;

    #[actix_rt::test]
    async fn ephemeral_sources() {
        let src = Ephemeral::new("foo::bar", "pattern bob");

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
            // Single-line comment, yay
            pattern signed-thing = {
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

        assert!(result.unwrap().satisfied())
        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_literals() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        pattern named<name> = {
            name: name
        }

        pattern jim = named<"Jim">
        pattern bob = named<"Bob">

        pattern folks = jim || bob

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

        assert!(runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Bob",
                        "age": 52,
                    }
                ),
            )
            .await
            .unwrap()
            .satisfied());
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_types() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
                pattern named<name> = {
                    name: name
                }

                pattern jim = named<integer>
                pattern bob = named<"Bob">

                pattern folks = jim || bob

                "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        assert!(runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Bob",
                        "age": 52,
                    }
                ),
            )
            .await
            .unwrap()
            .satisfied());
    }

    #[actix_rt::test]
    async fn evaluate_matches() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        pattern bob = {
            name: "Bob",
            age: $(self > 48),
        }

        pattern jim = {
            name: "Jim",
            age: $(self > 52),
        }

        pattern folks = bob || jim

        "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        assert!(runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Bob",
                        "age": 49,
                    }
                ),
            )
            .await
            .unwrap()
            .satisfied());

        assert!(!runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Jim",
                        "age": 49,
                    }
                )
            )
            .await
            .unwrap()
            .satisfied());

        assert!(!runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Bob",
                        "age": 42,
                    }
                )
            )
            .await
            .unwrap()
            .satisfied());

        assert!(runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Jim",
                        "age": 53,
                    }
                )
            )
            .await
            .unwrap()
            .satisfied());
    }
}
