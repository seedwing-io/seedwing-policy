//use crate::lang::expr::expr;
use crate::lang::hir::PatternDefn;
use crate::lang::parser::ty::{pkg_doc_comment, simple_type_name, type_definition, type_name};
use crate::runtime::PackagePath;
use crate::runtime::PatternName;
use chumsky::prelude::*;
use chumsky::{Error, Parser, Stream};

use serde::{Serialize, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use std::ops::{Deref, DerefMut};

pub mod expr;
pub mod literal;
pub mod meta;
pub mod ty;

pub type SourceSpan = std::ops::Range<usize>;

#[derive(Debug)]
pub struct CompilationUnit {
    source: SourceLocation,
    uses: Vec<Located<Use>>,
    types: Vec<Located<PatternDefn>>,
    documentation: Option<String>,
}

impl CompilationUnit {
    pub fn new(source: SourceLocation) -> Self {
        Self {
            source,
            uses: Default::default(),
            types: Default::default(),
            documentation: None,
        }
    }

    pub fn source(&self) -> SourceLocation {
        self.source.clone()
    }

    pub fn add_use(&mut self, ty: Located<Use>) {
        self.uses.push(ty)
    }

    pub fn add_type(&mut self, ty: Located<PatternDefn>) {
        self.types.push(ty)
    }

    pub(crate) fn uses(&self) -> &Vec<Located<Use>> {
        &self.uses
    }

    pub(crate) fn types(&self) -> &Vec<Located<PatternDefn>> {
        &self.types
    }

    pub(crate) fn types_mut(&mut self) -> &mut Vec<Located<PatternDefn>> {
        &mut self.types
    }

    pub(crate) fn documentation(&self) -> Option<&str> {
        self.documentation.as_deref()
    }
}

#[derive(Debug)]
pub struct Use {
    type_path: Located<PatternName>,
    as_name: Option<Located<String>>,
}

impl Use {
    pub fn new(type_path: Located<PatternName>, as_name: Option<Located<String>>) -> Self {
        Self { type_path, as_name }
    }

    pub fn type_name(&self) -> Located<PatternName> {
        self.type_path.clone()
    }

    pub fn as_name(&self) -> Located<String> {
        if let Some(as_name) = &self.as_name {
            as_name.clone()
        } else {
            Located::new(self.type_path.name().to_string(), self.type_path.location())
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Location {
    span: SourceSpan,
}

impl Location {
    pub fn span(&self) -> SourceSpan {
        self.span.clone()
    }
}

impl From<SourceSpan> for Location {
    fn from(span: SourceSpan) -> Self {
        Self { span }
    }
}

pub struct Located<T> {
    inner: T,
    location: Location,
}

impl<T: Serialize> Serialize for Located<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<T: Debug> Debug for Located<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

impl<T: Eq + PartialEq> Eq for Located<T> {}

impl<T: PartialEq> PartialEq for Located<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

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

impl<T: Clone> Located<T> {
    pub fn inner(&self) -> T {
        self.inner.clone()
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

    pub fn span(&self) -> SourceSpan {
        self.location.span.clone()
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn split(self) -> (T, Location) {
        (self.inner, self.location)
    }

    pub fn map<F, U>(self, f: F) -> Located<U>
    where
        F: FnOnce(T) -> U,
    {
        Located::new(f(self.inner), self.location)
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

#[derive(Hash, PartialEq, Eq, PartialOrd, Debug, Clone)]
pub struct SourceLocation {
    name: String,
}

impl SourceLocation {
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl From<SourceLocation> for String {
    fn from(loc: SourceLocation) -> Self {
        loc.name
    }
}

impl Display for SourceLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<String> for SourceLocation {
    fn from(name: String) -> Self {
        Self { name }
    }
}

impl From<&str> for SourceLocation {
    fn from(name: &str) -> Self {
        Self { name: name.into() }
    }
}

impl From<PackagePath> for SourceLocation {
    fn from(package: PackagePath) -> Self {
        SourceLocation {
            name: package
                .path
                .into_iter()
                .map(|e| e.0)
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
        Src: Into<SourceLocation> + Clone,
        S: Into<Stream<'a, ParserInput, <ParserError as Error<ParserInput>>::Span, Iter>>,
    {
        let tokens = lexer().parse(stream)?;
        let tokens = remove_comments(&tokens);

        //let bytes = tokens.iter().map(|e|e.0).collect::<String>();
        //println!("{:?}", bytes);

        let (compilation_unit, errors) = compilation_unit(source).parse_recovery_verbose(
            Stream::from_iter(tokens.len()..tokens.len() + 1, tokens.iter().cloned()),
        );

        if !errors.is_empty() {
            Err(errors)
        } else if let Some(compilation_unit) = compilation_unit {
            Ok(compilation_unit)
        } else {
            Err(vec![ParserError::custom(
                0..0,
                "Unable to parse; no further details available",
            )])
        }
    }
}

fn remove_comments(tokens: &Vec<(ParserInput, SourceSpan)>) -> Vec<(ParserInput, SourceSpan)> {
    let mut filtered_tokens = Vec::new();
    let len = tokens.len();

    let mut i = 0;
    let mut inside_string = false;
    loop {
        if i >= len {
            break;
        }
        if tokens[i].0 == '"' {
            filtered_tokens.push(tokens[i].clone());
            inside_string = !inside_string;
        } else if tokens[i].0 == '/' && !inside_string {
            if tokens[i + 1].0 == '/' {
                match tokens[i + 2].0 {
                    '/' | '!' => {
                        filtered_tokens.push(tokens[i].clone());
                        filtered_tokens.push(tokens[i + 1].clone());
                        filtered_tokens.push(tokens[i + 2].clone());
                        i += 2;
                    }
                    _ => {
                        i += 2;
                        // consume until newline
                        while tokens[i].0 != '\n' {
                            i += 1;
                        }
                    }
                }
            } else {
                filtered_tokens.push(tokens[i].clone())
            }
        } else {
            filtered_tokens.push(tokens[i].clone())
        }
        i += 1;
    }

    /*
    let debug: String = filtered_tokens.iter().map(|e| {
        e.0
    }).collect();

    println!("{}", debug);
     */

    filtered_tokens
}

pub fn lexer(
) -> impl Parser<ParserInput, Vec<(ParserInput, SourceSpan)>, Error = ParserError> + Clone {
    any().map_with_span(|l, span| (l, span)).repeated()
}

fn op(op: &str) -> impl Parser<ParserInput, &str, Error = ParserError> + Clone {
    just(op).padded()
}

pub fn use_statement() -> impl Parser<ParserInput, Located<Use>, Error = ParserError> + Clone {
    just("use")
        .padded()
        .ignored()
        .then(type_name())
        .then(as_clause().or_not())
        // .then( just(";").padded().ignored() )
        .map_with_span(|((_, type_path), as_clause), span| {
            Located::new(Use::new(type_path, as_clause), span)
        })
}

pub fn as_clause() -> impl Parser<ParserInput, Located<String>, Error = ParserError> + Clone {
    just("as")
        .padded()
        .ignored()
        .then(simple_type_name())
        .map(|(_, v)| v)
}

pub fn compilation_unit<S>(
    source: S,
) -> impl Parser<ParserInput, CompilationUnit, Error = ParserError> + Clone
where
    S: Into<SourceLocation> + Clone,
{
    pkg_doc_comment(0)
        .padded()
        .then(use_statement().padded().repeated())
        .then(type_definition().padded().repeated())
        .then_ignore(end())
        .map(move |((pkg_doc, use_statements), types)| {
            let mut unit = CompilationUnit::new(source.clone().into());

            unit.documentation = if !pkg_doc.is_empty() {
                Some(pkg_doc)
            } else {
                None
            };

            for e in use_statements {
                unit.add_use(e)
            }

            for e in types {
                unit.add_type(e)
            }

            unit
        })
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

#[cfg(test)]
mod test {
    use crate::lang::parser::Located;

    /// created a located instance suitable for testing only (as it has a range of 0..0)
    pub(crate) fn located<T>(inner: impl Into<T>) -> Located<T> {
        Located::new(inner.into(), 0..0usize)
    }
}
