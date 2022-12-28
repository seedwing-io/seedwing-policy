use crate::lang::hir::Type;
use crate::lang::parser::{Located, ParserError, ParserInput, SourceSpan};
use crate::value::Value;
use chumsky::primitive::{filter, just};
use chumsky::text::TextParser;
use chumsky::{text, Parser};

pub fn integer_literal() -> impl Parser<ParserInput, Located<Value>, Error = ParserError> + Clone {
    text::int::<char, ParserError>(10)
        .padded()
        .map_with_span(|s: String, span| Located::new(s.parse::<i64>().unwrap().into(), span))
}

pub fn decimal_literal() -> impl Parser<ParserInput, Located<Value>, Error = ParserError> + Clone {
    text::int(10)
        .then(just('.').then(text::int(10)))
        .padded()
        .map_with_span(
            |(integral, (_dot, decimal)): (String, (char, String)), span| {
                Located::new(
                    format!("{}.{}", integral, decimal)
                        .parse::<f64>()
                        .unwrap()
                        .into(),
                    span,
                )
            },
        )
}

pub fn string_literal() -> impl Parser<ParserInput, Located<Value>, Error = ParserError> + Clone {
    just('"')
        .ignored()
        .then(filter(|c: &char| *c != '"').repeated().collect::<String>())
        .then(just('"').ignored())
        .padded()
        .map_with_span(|((_, x), _), span: SourceSpan| Located::new(x.into(), span))
}

pub fn anything_literal() -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("anything")
        .padded()
        .ignored()
        .map_with_span(|_, span| Located::new(Type::Anything, span))
}
