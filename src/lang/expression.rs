use chumsky::Parser;
use chumsky::prelude::*;
use crate::lang::{ParserError, ParserInput, Spanned, Value};

#[derive(Clone, Debug)]
pub enum Expr {
    SelfLiteral,
    True,
    False,

    LessThan(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    LessThanEqual(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    GreaterThan(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    GreaterThanEqual(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Equal(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Inequal(Box<Spanned<Expr>>, Box<Spanned<Expr>>),

    And(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Or(Box<Spanned<Expr>>, Box<Spanned<Expr>>),

    Value(Value),
    Negative(Box<Spanned<Expr>>),
    Add(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Subtract(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Multiply(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Divide(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    //Type(TypeName),
}

pub fn op(
    text: &'static str,
) -> impl Parser<ParserInput, Spanned<String>, Error=ParserError> + Clone {
    just(text)
        .map_with_span(|v, span| (v.to_string(), span))
        .padded()
}

pub fn integer_expr() -> impl Parser<ParserInput, Spanned<Expr>, Error=ParserError> + Clone {
    let value = text::int::<char, ParserError>(10)
        .map(|s: String| Value::Integer(s.parse().unwrap()))
        .padded();

    value.map_with_span(|value, span| (Expr::Value(value), span))
}

pub fn decimal_expr() -> impl Parser<ParserInput, Spanned<Expr>, Error=ParserError> + Clone {
    let value = text::int(10)
        .then(just('.').then(text::int(10)))
        .map(|(integral, (_dot, decimal)): (String, (char, String))| {
            Value::Decimal(format!("{}.{}", integral, decimal).parse().unwrap())
        })
        .padded();

    value.map_with_span(|value, span| (Expr::Value(value), span))
}

pub fn self_literal() -> impl Parser<ParserInput, Spanned<Expr>, Error=ParserError> + Clone {
    just("self").map_with_span(|v, span| {
        (Expr::SelfLiteral, span)
    })
}

pub fn atom() -> impl Parser<ParserInput, Spanned<Expr>, Error=ParserError> + Clone {
    self_literal()
        .or(decimal_expr())
        .or(integer_expr())
}

pub fn expr() -> impl Parser<ParserInput, Spanned<Expr>, Error=ParserError> + Clone {
    recursive(|expr| {
        logical(expr)
    })
}

pub fn logical(expr: impl Parser<ParserInput, Spanned<Expr>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Spanned<Expr>, Error=ParserError> + Clone {
    relational(expr.clone())
        .then(
            (op("&&").to(Expr::And as fn(_, _) -> Expr)
                .or(op("||").to(Expr::Or as fn(_, _) -> Expr))
                .then(expr))
                .repeated()
                .at_most(1),
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.1.start()..rhs.1.end();
            (op(Box::new(lhs), Box::new(rhs)), span)
        })
        .padded()
}

pub fn relational(expr: impl Parser<ParserInput, Spanned<Expr>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Spanned<Expr>, Error=ParserError> + Clone {
    atom()
        .then(
            (op(">")
                .to(Expr::GreaterThan as fn(_, _) -> Expr)
                .or(op(">=").to(Expr::GreaterThanEqual as fn(_, _) -> Expr))
                .or(op("<").to(Expr::LessThan as fn(_, _) -> Expr))
                .or(op("<=").to(Expr::LessThanEqual as fn(_, _) -> Expr))
                .or(op("!=").to(Expr::Inequal as fn(_, _) -> Expr))
                .or(op("==").to(Expr::Equal as fn(_, _) -> Expr))
                .then(expr))
                .repeated()
                .at_most(1),
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.1.start()..rhs.1.end();
            (op(Box::new(lhs), Box::new(rhs)), span)
        })
        .padded()
}
