use crate::lang::hir::Type;
use crate::lang::lir::{ValueType, ID_COUNTER};
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
use std::sync::Arc;

#[derive(Serialize, Debug, Clone)]
pub struct Expr {
    pub(crate) id: u64,
    inner: InnerExpr,
}

impl Expr {
    pub fn new(inner: InnerExpr) -> Self {
        Self {
            id: ID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed),
            inner,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub enum InnerExpr {
    SelfLiteral(#[serde(skip)] Location),
    /* self */
    Value(Located<ValueType>),
    Accessor(Arc<Located<Expr>>, Located<String>),
    Field(Arc<Located<Expr>>, Arc<Located<Expr>>),
    /* self.len */
    Function(Located<String>, Arc<Located<Expr>>),
    /* len(self) */
    Add(Arc<Located<Expr>>, Arc<Located<Expr>>),
    Subtract(Arc<Located<Expr>>, Arc<Located<Expr>>),
    Multiply(Arc<Located<Expr>>, Arc<Located<Expr>>),
    Divide(Arc<Located<Expr>>, Arc<Located<Expr>>),
    LessThan(Arc<Located<Expr>>, Arc<Located<Expr>>),
    LessThanEqual(Arc<Located<Expr>>, Arc<Located<Expr>>),
    GreaterThan(Arc<Located<Expr>>, Arc<Located<Expr>>),
    GreaterThanEqual(Arc<Located<Expr>>, Arc<Located<Expr>>),
    Equal(Arc<Located<Expr>>, Arc<Located<Expr>>),
    NotEqual(Arc<Located<Expr>>, Arc<Located<Expr>>),
    Not(Arc<Located<Expr>>),
    LogicalAnd(Arc<Located<Expr>>, Arc<Located<Expr>>),
    LogicalOr(Arc<Located<Expr>>, Arc<Located<Expr>>),
}

pub type ExprFuture =
    Pin<Box<dyn Future<Output = Result<Rc<RuntimeValue>, RuntimeError>> + 'static>>;

impl Located<Expr> {
    #[allow(clippy::let_and_return)]
    pub fn evaluate(self: &Arc<Self>, value: Rc<RuntimeValue>) -> ExprFuture {
        let this = self.clone();

        Box::pin(async move {
            match &this.inner.inner {
                InnerExpr::SelfLiteral(_) => Ok(value.clone()),
                InnerExpr::Value(ref inner) => Ok(Rc::new((&inner.inner()).into())),
                InnerExpr::Accessor(_, _) => todo!(),
                InnerExpr::Field(_, _) => todo!(),
                InnerExpr::Function(_, _) => todo!(),
                InnerExpr::Add(_, _) => todo!(),
                InnerExpr::Subtract(_, _) => todo!(),
                InnerExpr::Multiply(_, _) => todo!(),
                InnerExpr::Divide(_, _) => todo!(),
                InnerExpr::LessThan(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Less) = (*lhs).partial_cmp(&(*rhs)) {
                        Ok(Rc::new(true.into()))
                    } else {
                        Ok(Rc::new(false.into()))
                    };

                    result
                }
                InnerExpr::LessThanEqual(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Less | Ordering::Equal) =
                        (*lhs).partial_cmp(&(*rhs))
                    {
                        Ok(Rc::new(true.into()))
                    } else {
                        Ok(Rc::new(false.into()))
                    };

                    result
                }
                InnerExpr::GreaterThan(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Greater) = (*lhs).partial_cmp(&(*rhs)) {
                        Ok(Rc::new(true.into()))
                    } else {
                        Ok(Rc::new(false.into()))
                    };

                    result
                }
                InnerExpr::GreaterThanEqual(lhs, rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Greater | Ordering::Equal) =
                        (*lhs).partial_cmp(&(*rhs))
                    {
                        Ok(Rc::new(true.into()))
                    } else {
                        Ok(Rc::new(false.into()))
                    };

                    result
                }
                InnerExpr::Equal(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Equal) = (*lhs).partial_cmp(&(*rhs)) {
                        Ok(Rc::new(true.into()))
                    } else {
                        Ok(Rc::new(false.into()))
                    };

                    result
                }
                InnerExpr::NotEqual(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Equal) = (*lhs).partial_cmp(&(*rhs)) {
                        Ok(Rc::new(false.into()))
                    } else {
                        Ok(Rc::new(true.into()))
                    };

                    result
                }
                InnerExpr::Not(_) => todo!(),
                InnerExpr::LogicalAnd(_, _) => todo!(),
                InnerExpr::LogicalOr(_, _) => todo!(),
            }
        })
    }
}

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
                Expr::new(InnerExpr::Value(Located::new(
                    ValueType::Boolean(true),
                    span.clone(),
                ))),
                span,
            )
        })
        .or(just("false").padded().map_with_span(|_, span: SourceSpan| {
            Located::new(
                Expr::new(InnerExpr::Value(Located::new(
                    ValueType::Boolean(false),
                    span.clone(),
                ))),
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
        .map_with_span(|value, span| Located::new(Expr::new(InnerExpr::Value(value)), span))
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
        .map_with_span(|value, span| Located::new(Expr::new(InnerExpr::Value(value)), span))
}

pub fn string_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    just('"')
        .ignored()
        .then(filter(|c: &char| *c != '"').repeated().collect::<String>())
        .then(just('"').ignored())
        .padded()
        .map_with_span(|((_, x), _), span: SourceSpan| {
            Located::new(
                Expr::new(InnerExpr::Value(Located::new(
                    ValueType::String(x),
                    span.clone(),
                ))),
                span,
            )
        })
}

pub fn self_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    just("self").padded().map_with_span(|v, span: SourceSpan| {
        Located::new(
            Expr::new(InnerExpr::SelfLiteral(Location::from(span.clone()))),
            span,
        )
    })
}

pub fn field_expr() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    text::ident()
        .map_with_span(Located::new)
        .then(op(":").padded().ignored())
        .then(expr())
        .map(|((field_name, _colon), expr)| {
            let primary_location = field_name.location();
            let expr_self = Located::new(
                Expr::new(InnerExpr::Accessor(
                    Arc::new(Located::new(
                        Expr::new(InnerExpr::SelfLiteral(primary_location.clone())),
                        primary_location.clone(),
                    )),
                    field_name,
                )),
                primary_location.clone(),
            );

            let field_location = primary_location.span().start()..expr.span().end();

            Located::new(
                Expr::new(InnerExpr::Field(Arc::new(expr_self), Arc::new(expr))),
                field_location,
            )
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
            Located::new(
                Expr::new(InnerExpr::LogicalOr(Arc::new(lhs), Arc::new(rhs))),
                span,
            )
        })
}

pub fn logical_and(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    relational_expr(expr.clone())
        .then(op("&&").then(expr).repeated())
        .foldl(|lhs, (_op, rhs)| {
            let span = lhs.span().start()..rhs.span().end();
            Located::new(
                Expr::new(InnerExpr::LogicalAnd(Arc::new(lhs), Arc::new(rhs))),
                span,
            )
        })
}

pub fn relational_expr(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    additive_expr(expr.clone())
        .then(
            op(">=")
                .map_with_span(|_, span| {
                    Located::new(InnerExpr::GreaterThanEqual as fn(_, _) -> _, span)
                })
                .or(op(">").map_with_span(|_, span| {
                    Located::new(InnerExpr::GreaterThan as fn(_, _) -> _, span)
                }))
                .or(op("<=").map_with_span(|_, span| {
                    Located::new(InnerExpr::LessThanEqual as fn(_, _) -> _, span)
                }))
                .or(op("<").map_with_span(|_, span| {
                    Located::new(InnerExpr::LessThan as fn(_, _) -> _, span)
                }))
                .or(op("==")
                    .map_with_span(|_, span| Located::new(InnerExpr::Equal as fn(_, _) -> _, span)))
                .or(op("!=").map_with_span(|_, span| {
                    Located::new(InnerExpr::NotEqual as fn(_, _) -> _, span)
                }))
                .then(expr)
                .or_not(),
        )
        .map(|(lhs, rhs)| {
            if let Some((op, rhs)) = rhs {
                let span = op.span().start()..rhs.span().end;
                Located::new(Expr::new(op(Arc::new(lhs), Arc::new(rhs))), span)
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
                .map_with_span(|_, span| Located::new(InnerExpr::Add as fn(_, _) -> _, span))
                .or(op("-").map_with_span(|_, span| {
                    Located::new(InnerExpr::Subtract as fn(_, _) -> _, span)
                }))
                .then(multiplicative_expr(expr))
                .repeated(),
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.span().start()..rhs.span().end;
            Located::new(Expr::new(op(Arc::new(lhs), Arc::new(rhs))), span)
        })
}

pub fn multiplicative_expr(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    atom()
        .then(
            op("*")
                .map_with_span(|_, span| Located::new(InnerExpr::Multiply as fn(_, _) -> _, span))
                .or(op("/").map_with_span(|_, span| {
                    Located::new(InnerExpr::Divide as fn(_, _) -> _, span)
                }))
                .then(atom())
                .repeated(),
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.span().start()..rhs.span().end;
            Located::new(Expr::new(op(Arc::new(lhs), Arc::new(rhs))), span)
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
