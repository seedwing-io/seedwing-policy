use crate::lang::hir::Type;
use crate::lang::lir::ValueType;
use crate::lang::parser::{Located, ParserError, ParserInput, SourceSpan};
use chumsky::primitive::{choice, filter, just};
use chumsky::text::TextParser;
use chumsky::{text, Parser};

pub fn integer_literal() -> impl Parser<ParserInput, Located<ValueType>, Error = ParserError> + Clone
{
    text::int::<char, ParserError>(10)
        .padded()
        .map_with_span(|s: String, span| {
            Located::new(ValueType::Integer(s.parse::<i64>().unwrap()), span)
        })
}

pub fn decimal_literal() -> impl Parser<ParserInput, Located<ValueType>, Error = ParserError> + Clone
{
    text::int(10)
        .then(just('.').then(text::int(10)))
        .padded()
        .map_with_span(
            |(integral, (_dot, decimal)): (String, (char, String)), span| {
                Located::new(
                    ValueType::Decimal(format!("{}.{}", integral, decimal).parse::<f64>().unwrap()),
                    span,
                )
            },
        )
}

pub fn boolean_literal() -> impl Parser<ParserInput, Located<ValueType>, Error = ParserError> + Clone
{
    choice((
        just("true").map(|_| ValueType::Boolean(true)),
        just("false").map(|_| ValueType::Boolean(false)),
    ))
    .map_with_span(Located::new)
}

pub fn string_literal() -> impl Parser<ParserInput, Located<ValueType>, Error = ParserError> + Clone
{
    just('"')
        .ignored()
        .then(filter(|c: &char| *c != '"').repeated().collect::<String>())
        .then(just('"').ignored())
        .padded()
        .map_with_span(|((_, x), _), span: SourceSpan| Located::new(ValueType::String(x), span))
}

/*
pub fn anything_literal() -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("anything")
        .padded()
        .ignored()
        .map_with_span(|_, span| Located::new(Type::Anything, span))
}

pub fn self_literal() -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("self")
        .padded()
        .ignored()
        .map_with_span(|_, span| Located::new(Type::Anything, span))
}
 */

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_boolean() {
        assert!(boolean_literal().parse("true").is_ok());
        assert!(boolean_literal().parse("false").is_ok());
        assert!(boolean_literal().parse("foo").is_err());
    }
}
