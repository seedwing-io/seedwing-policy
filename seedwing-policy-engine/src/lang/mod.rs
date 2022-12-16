//use crate::lang::expr::expr;
use crate::lang::ty::{compilation_unit, PackagePath, Type, TypeDefn, TypeName};
use crate::runtime::BuildError;
use chumsky::prelude::*;
use chumsky::{Error, Parser, Stream};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

pub mod expr;
pub mod ty;

pub type Span = std::ops::Range<usize>;

#[derive(Debug)]
pub struct CompilationUnit {
    source: Source,
    uses: Vec<Located<Use>>,
    types: Vec<Located<TypeDefn>>,
}

impl CompilationUnit {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            uses: Default::default(),
            types: Default::default(),
        }
    }

    pub fn source(&self) -> Source {
        self.source.clone()
    }

    pub fn add_use(&mut self, ty: Located<Use>) {
        self.uses.push(ty)
    }

    pub fn add_type(&mut self, ty: Located<TypeDefn>) {
        self.types.push(ty)
    }

    pub(crate) fn uses(&self) -> &Vec<Located<Use>> {
        &self.uses
    }

    pub(crate) fn types(&self) -> &Vec<Located<TypeDefn>> {
        &self.types
    }

    pub(crate) fn types_mut(&mut self) -> &mut Vec<Located<TypeDefn>> {
        &mut self.types
    }
}

#[derive(Debug)]
pub struct Use {
    type_path: Located<TypeName>,
    as_name: Option<Located<String>>,
}

impl Use {
    pub fn new(type_path: Located<TypeName>, as_name: Option<Located<String>>) -> Self {
        Self { type_path, as_name }
    }

    pub fn type_name(&self) -> Located<TypeName> {
        self.type_path.clone()
    }

    pub fn as_name(&self) -> Located<String> {
        if let Some(as_name) = &self.as_name {
            as_name.clone()
        } else {
            Located::new(self.type_path.name().clone(), self.type_path.location())
        }
    }
}

#[derive(Clone, Debug)]
pub struct Location {
    span: Span,
}

impl Location {
    pub fn span(&self) -> Span {
        self.span.clone()
    }
}

impl From<Span> for Location {
    fn from(span: Span) -> Self {
        Self { span }
    }
}

pub struct Located<T> {
    inner: T,
    location: Location,
}

impl<T: Debug> Debug for Located<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

impl<T: Eq + PartialEq> Eq for Located<T> {}

impl<T: PartialEq> PartialEq for Located<T> {
    fn eq(&self, other: &Self) -> bool {
        (&self.inner).eq(&other.inner)
    }
}

/*
impl<T: Eq> Eq for Located<T> {

}

 */

/*
impl<T: PartialEq> PartialEq<Self> for Located<T> {
    fn eq(&self, other: &Self) -> bool {
        ////self.inner.eq( &other.inner )
        PartialEq::eq( &self.inner, &other.inner )
    }
}

impl<T: Eq + PartialEq<T>> Eq for Located<T> {

}
 */

impl<T: Hash> Hash for Located<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<T: Clone> Clone for Located<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            location: self.location.clone(),
        }
    }
}

impl<T> Located<T> {
    pub fn new<L: Into<Location>>(inner: T, location: L) -> Self {
        Self {
            location: location.into(),
            inner,
        }
    }

    pub fn location(&self) -> Location {
        self.location.clone()
    }

    pub fn span(&self) -> Span {
        self.location.span.clone()
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn split(self) -> (T, Location) {
        (self.inner, self.location)
    }
}

impl<T> Deref for Located<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Located<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[allow(unused)]
pub type ParserInput = char;
#[allow(unused)]
pub type ParserError = Simple<char>;

#[derive(Clone, Debug)]
pub struct FieldName(String);

impl FieldName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn name(&self) -> String {
        self.0.clone()
    }
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Debug, Clone)]
pub struct Source {
    name: String,
}

impl From<String> for Source {
    fn from(name: String) -> Self {
        Self { name }
    }
}

impl From<&str> for Source {
    fn from(name: &str) -> Self {
        Self { name: name.into() }
    }
}

impl From<PackagePath> for Source {
    fn from(package: PackagePath) -> Self {
        Source {
            name: package
                .path()
                .iter()
                .map(|e| (*(e.clone().into_inner())).clone())
                .collect::<Vec<String>>()
                .join("/"),
        }
    }
}

#[derive(Copy, Clone, Default)]
pub struct PolicyParser {}

impl PolicyParser {
    pub fn parse<'a, Iter, Src, S>(
        &self,
        source: Src,
        stream: S,
    ) -> Result<CompilationUnit, Vec<ParserError>>
    where
        Self: Sized,
        Iter: Iterator<Item = (ParserInput, <ParserError as Error<ParserInput>>::Span)> + 'a,
        Src: Into<Source> + Clone,
        S: Into<Stream<'a, ParserInput, <ParserError as Error<ParserInput>>::Span, Iter>>,
    {
        Ok(compilation_unit(source).parse(stream)?)
    }
}

/*
#[cfg(test)]
mod test {
    use super::*;
    use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
    use crate::lang::ty::type_name;

    #[test]
    fn parse_logical() {
        let result = expr().parse(r#"
            Tall && Tired
        "#);

        println!("{:?}", result);
    }

    #[test]
    fn parse_type_name() {
        let result = type_name().parse("Bob").unwrap();
        assert_eq!("Bob", result.0.0);

        let result = type_name().parse("bob");
        assert!(matches!( result, Err(_)));
    }

    #[test]
    fn parse_expr_type() {
        let result = ty().parse(r#"
            type Bob := Tall && DogOwner;
        "#).unwrap();

        assert_eq!("Bob", result.0.name);
    }

    #[test]
    fn parse_object_type() {
        let result = ty().parse(r#"
        type Bob := {
            age: this > 49,
            name: this < 23,
        }
        "#).unwrap();

        println!("{:?}", result);

        assert_eq!("Bob", result.0.name);
    }

    #[test]
    fn parse_decimal() {
        let parser = PolicyParser::default();
        let src = "\n\n42.8821 == \n\t42";
        let result = parser.parse(src);

        match result {
            Err(errors) => errors
                .iter()
                .cloned()
                .map(|e| e.map(|e| e.to_string()))
                .for_each(|e| {
                    let report = Report::build(ReportKind::Error, (), e.span().start);

                    let report = match e.reason() {
                        chumsky::error::SimpleReason::Unclosed { span, delimiter } => report
                            .with_message(format!(
                                "Unclosed delimiter {}",
                                delimiter.fg(Color::Yellow)
                            ))
                            .with_label(
                                Label::new(span.clone())
                                    .with_message(format!(
                                        "Unclosed delimiter {}",
                                        delimiter.fg(Color::Yellow)
                                    ))
                                    .with_color(Color::Yellow),
                            )
                            .with_label(
                                Label::new(e.span())
                                    .with_message(format!(
                                        "Must be closed before this {}",
                                        e.found()
                                            .unwrap_or(&"end of file".to_string())
                                            .fg(Color::Red)
                                    ))
                                    .with_color(Color::Red),
                            ),
                        chumsky::error::SimpleReason::Unexpected => report
                            .with_message(format!(
                                "{}, expected {}",
                                if e.found().is_some() {
                                    "Unexpected token in input"
                                } else {
                                    "Unexpected end of input"
                                },
                                if e.expected().len() == 0 {
                                    "something else".to_string()
                                } else {
                                    e.expected()
                                        .map(|expected| match expected {
                                            Some(expected) => expected.to_string(),
                                            None => "end of input".to_string(),
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                }
                            ))
                            .with_label(
                                Label::new(e.span())
                                    .with_message(format!(
                                        "Unexpected token {}",
                                        e.found()
                                            .unwrap_or(&"end of file".to_string())
                                            .fg(Color::Red)
                                    ))
                                    .with_color(Color::Red),
                            ),
                        chumsky::error::SimpleReason::Custom(msg) => {
                            report.with_message(msg).with_label(
                                Label::new(e.span())
                                    .with_message(format!("{}", msg.fg(Color::Red)))
                                    .with_color(Color::Red),
                            )
                        }
                    };

                    report.finish().print(Source::from(&src)).unwrap();
                }),

            Ok(parsed) => {
                println!("{:?}", parsed)
            }
        }
    }
}
 */
