use crate::lang::hir::{Expr, Type};
use crate::lang::lir::ValueType;
use crate::lang::parser::{FieldName, Located, Location, ParserError, ParserInput, SourceSpan};
use crate::runtime::RuntimeError;
use crate::value::RuntimeValue;
use chumsky::prelude::*;
use chumsky::Parser;
use serde::Serialize;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::future::{ready, Future};
use std::pin::Pin;
use std::rc::Rc;

#[derive(Copy, Clone, Debug)]
pub enum ExprError {
    Value(ValueError),
    Simplify,
}

impl From<ValueError> for ExprError {
    fn from(inner: ValueError) -> Self {
        Self::Value(inner)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ValueError {
    NonArithmatic,
    DivideByZero,
}

pub fn op(op: &str) -> impl Parser<ParserInput, &str, Error = ParserError> + Clone {
    just(op).padded()
}

pub fn boolean_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    just("true")
        .padded()
        .map_with_span(|_, span: SourceSpan| {
            Located::new(
                Expr::Value(Located::new(ValueType::Boolean(true), span.clone())),
                span,
            )
        })
        .or(just("false").padded().map_with_span(|_, span: SourceSpan| {
            Located::new(
                Expr::Value(Located::new(ValueType::Boolean(false), span.clone())),
                span,
            )
        }))
}

pub fn integer_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    text::int::<char, ParserError>(10)
        .map_with_span(|s: String, span| {
            Located::new(ValueType::Integer(s.parse::<i64>().unwrap()), span)
        })
        .padded()
        .map_with_span(|value, span| Located::new(Expr::Value(value), span))
}

pub fn decimal_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
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
        .map_with_span(|value, span| Located::new(Expr::Value(value), span))
}

pub fn string_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    just('"')
        .ignored()
        .then(filter(|c: &char| *c != '"').repeated().collect::<String>())
        .then(just('"').ignored())
        .padded()
        .map_with_span(|((_, x), _), span: SourceSpan| {
            Located::new(
                Expr::Value(Located::new(ValueType::String(x), span.clone())),
                span,
            )
        })
}

pub fn self_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    just("self").padded().map_with_span(|v, span: SourceSpan| {
        Located::new(Expr::SelfLiteral(Location::from(span.clone())), span)
    })
}

pub fn atom() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    self_literal()
        .or(string_literal())
        .or(decimal_literal())
        .or(integer_literal())
        .or(boolean_literal())
}

pub fn expr() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    recursive(|expr| parenthesized_expr(expr.clone()).or(logical_or(expr)))
}

pub fn parenthesized_expr(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    just("(")
        .padded()
        .ignored()
        .then(expr)
        .then(just(")").padded().ignored())
        .map(|((_left_paren, expr), _right_paren)| expr)
}

pub fn logical_or(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    logical_and(expr.clone())
        .then(op("||").then(expr).repeated())
        .foldl(|lhs, (_op, rhs)| {
            let span = lhs.span().start()..rhs.span().end();
            Located::new(Expr::LogicalOr(Box::new(lhs), Box::new(rhs)), span)
        })
}

pub fn logical_and(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    relational_expr(expr.clone())
        .then(op("&&").then(expr).repeated())
        .foldl(|lhs, (_op, rhs)| {
            let span = lhs.span().start()..rhs.span().end();
            Located::new(Expr::LogicalAnd(Box::new(lhs), Box::new(rhs)), span)
        })
}

pub fn relational_expr(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    additive_expr(expr.clone())
        .then(
            op(">=")
                .map_with_span(|_, span| {
                    Located::new(Expr::GreaterThanEqual as fn(_, _) -> _, span)
                })
                .or(op(">").map_with_span(|_, span| {
                    Located::new(Expr::GreaterThan as fn(_, _) -> _, span)
                }))
                .or(op("<=").map_with_span(|_, span| {
                    Located::new(Expr::LessThanEqual as fn(_, _) -> _, span)
                }))
                .or(op("<")
                    .map_with_span(|_, span| Located::new(Expr::LessThan as fn(_, _) -> _, span)))
                .or(op("==")
                    .map_with_span(|_, span| Located::new(Expr::Equal as fn(_, _) -> _, span)))
                .or(op("!=")
                    .map_with_span(|_, span| Located::new(Expr::NotEqual as fn(_, _) -> _, span)))
                .then(expr)
                .or_not(),
        )
        .map(|(lhs, rhs)| {
            if let Some((op, rhs)) = rhs {
                let span = op.span().start()..rhs.span().end;
                Located::new(op(Box::new(lhs), Box::new(rhs)), span)
            } else {
                lhs
            }
        })
}

pub fn additive_expr(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    multiplicative_expr(expr.clone())
        .then(
            op("+")
                .map_with_span(|_, span| Located::new(Expr::Add as fn(_, _) -> _, span))
                .or(op("-")
                    .map_with_span(|_, span| Located::new(Expr::Subtract as fn(_, _) -> _, span)))
                .then(multiplicative_expr(expr))
                .repeated(),
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.span().start()..rhs.span().end;
            Located::new(op(Box::new(lhs), Box::new(rhs)), span)
        })
}

pub fn multiplicative_expr(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    atom()
        .then(
            op("*")
                .map_with_span(|_, span| Located::new(Expr::Multiply as fn(_, _) -> _, span))
                .or(op("/")
                    .map_with_span(|_, span| Located::new(Expr::Divide as fn(_, _) -> _, span)))
                .then(atom())
                .repeated(),
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.span().start()..rhs.span().end;
            Located::new(op(Box::new(lhs), Box::new(rhs)), span)
        })
}

#[cfg(test)]
mod test {
    use super::*;

    /*
    #[test]
    fn parse_self() {
        let ty = expr()
            .parse(
                r#"
            self
        "#,
            )
            .unwrap()
            .inner();

        assert!(matches!(ty, Expr::SelfLiteral(_)));
    }

    #[test]
    fn parse_string() {
        let ty = expr()
            .parse(
                r#"
                    "howdy"
            "#,
            )
            .unwrap()
            .inner();

        println!("{:?}", ty);
    }
     */

    /*
    #[test]
    fn parse_integer_literal() {
        let ty = expr()
            .parse(
                r#"
            42
        "#,
            )
            .unwrap()
            .into_inner();

        assert!(matches!(
            ty,
            Expr::Value(Located {
                inner: 42.into(),
                ..
            })
        ));
    }
     */

    #[test]
    fn parse_decimal_literal() {
        let ty = expr()
            .parse(
                r#"
            42.1415
        "#,
            )
            .unwrap()
            .inner();

        /*
        assert!(matches!( ty,
            Expr::Value(
                Located {
                    inner:  Value::Decimal(x),
                    ..
                } )
            if x > 42.1 && x < 42.2));
         */
    }

    /*
    #[test]
    fn parse_parenthesized_expr() {
        let value = expr()
            .parse(
                r#"
            (42 + 88)
        "#,
            )
            .unwrap()
            .into_inner();

        let value = value.simplify_expr().unwrap();

        println!("{:?}", value);

        assert!(matches!(
            value,
            Expr::Value(Located {
                inner: Value::Integer(130),
                ..
            })
        ));
    }
     */

    /*
    #[test]
    fn parse_math() {
        let value = expr()
            .parse(
                r#"
            1 + 2 * 3 + 4
        "#,
            )
            .unwrap()
            .into_inner();

        let value = value.simplify_expr().unwrap();

        println!("{:?}", value);

        assert!(matches!(
            value,
            Expr::Value(Located {
                inner: Value::Integer(11),
                ..
            })
        ));
    }
     */

    /*
    #[test]
    fn simplify_logical_or() {
        let value = expr()
            .parse(
                r#"
            false || true
        "#,
            )
            .unwrap()
            .into_inner();

        let value = value.simplify_expr().unwrap();

        assert!(matches!(
            value,
            Expr::Value(Located {
                inner: Value::Boolean(true),
                ..
            })
        ));

        let value = expr()
            .parse(
                r#"
            true || false
        "#,
            )
            .unwrap()
            .into_inner();

        let value = value.simplify_expr().unwrap();

        assert!(matches!(
            value,
            Expr::Value(Located {
                inner: Value::Boolean(true),
                ..
            })
        ));

        let value = expr()
            .parse(
                r#"
            true || true
        "#,
            )
            .unwrap()
            .into_inner();

        let value = value.simplify_expr().unwrap();

        assert!(matches!(
            value,
            Expr::Value(Located {
                inner: Value::Boolean(true),
                ..
            })
        ));

        let value = expr()
            .parse(
                r#"
            false || false
        "#,
            )
            .unwrap()
            .into_inner();

        let value = value.simplify_expr().unwrap();

        assert!(matches!(
            value,
            Expr::Value(Located {
                inner: Value::Boolean(false),
                ..
            })
        ));
    }
     */
}
