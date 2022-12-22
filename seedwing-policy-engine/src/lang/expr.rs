use crate::lang::ty::Type;
use crate::lang::{FieldName, Located, Location, ParserError, ParserInput, SourceSpan};
use crate::runtime::{RuntimeError, RuntimeType};
use crate::value::{Value as RuntimeValue, Value};
use async_mutex::Mutex;
use chumsky::prelude::*;
use chumsky::Parser;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::future::{ready, Future};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Expr {
    SelfLiteral(Location),
    /* self */
    Value(Located<Value>),
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
    Pin<Box<dyn Future<Output = Result<Arc<Mutex<RuntimeValue>>, RuntimeError>> + 'static>>;

impl Located<Expr> {
    pub fn evaluate(self: &Arc<Self>, value: Arc<Mutex<RuntimeValue>>) -> ExprFuture {
        let this = self.clone();

        Box::pin(async move {
            match &**this {
                Expr::SelfLiteral(_) => Ok(value.clone()),
                Expr::Value(ref inner) => Ok(Arc::new(Mutex::new(inner.clone().into_inner()))),
                Expr::Accessor(_, _) => todo!(),
                Expr::Field(_, _) => todo!(),
                Expr::Function(_, _) => todo!(),
                Expr::Add(_, _) => todo!(),
                Expr::Subtract(_, _) => todo!(),
                Expr::Multiply(_, _) => todo!(),
                Expr::Divide(_, _) => todo!(),
                Expr::LessThan(_, _) => todo!(),
                Expr::LessThanEqual(_, _) => todo!(),
                Expr::GreaterThan(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    #[allow(clippy::let_and_return)]
                    let result = if let Some(Ordering::Greater) =
                        lhs.lock().await.partial_cmp(&*rhs.lock().await)
                    {
                        Ok(Arc::new(Mutex::new(true.into())))
                    } else {
                        Ok(Arc::new(Mutex::new(false.into())))
                    };

                    result
                }
                Expr::GreaterThanEqual(_, _) => todo!(),
                Expr::Equal(_, _) => todo!(),
                Expr::NotEqual(_, _) => todo!(),
                Expr::Not(_) => todo!(),
                Expr::LogicalAnd(_, _) => todo!(),
                Expr::LogicalOr(_, _) => todo!(),
            }
        })
    }
}

impl Expr {
    pub fn is_constant(&self) -> bool {
        match self {
            Expr::SelfLiteral(_) => false,
            Expr::Value(_) => true,
            Expr::Accessor(lhs, _) => lhs.is_constant(),
            Expr::Function(_, operand) => operand.is_constant(),
            Expr::Add(lhs, rhs)
            | Expr::Subtract(lhs, rhs)
            | Expr::Multiply(lhs, rhs)
            | Expr::Divide(lhs, rhs)
            | Expr::LessThan(lhs, rhs)
            | Expr::LessThanEqual(lhs, rhs)
            | Expr::GreaterThan(lhs, rhs)
            | Expr::GreaterThanEqual(lhs, rhs)
            | Expr::LogicalAnd(lhs, rhs)
            | Expr::LogicalOr(lhs, rhs)
            | Expr::NotEqual(lhs, rhs)
            | Expr::Equal(lhs, rhs) => lhs.is_constant() && rhs.is_constant(),
            Expr::Not(v) => v.is_constant(),
            Expr::Field(this, _) => this.is_constant(),
        }
    }

    /*
    pub fn simplify_expr(&self) -> Result<Self, ExprError> {
        match self {
            Expr::SelfLiteral(_) => Ok(self.clone()),
            Expr::Value(_) => Ok(self.clone()),
            Expr::Add(lhs, rhs) => {
                let lhs = lhs.simplify_expr()?;
                let rhs = rhs.simplify_expr()?;

                match (lhs, rhs) {
                    (Expr::Value(lhs), Expr::Value(rhs)) => {
                        let location = lhs.span().start..rhs.span().end();
                        let value = lhs.try_add(&*rhs)?;
                        Ok(Expr::Value(Located::new(value, location.clone())))
                    }
                    _ => Ok(self.clone()),
                }
            }
            Expr::Subtract(lhs, rhs) => {
                let lhs = lhs.simplify_expr()?;
                let rhs = rhs.simplify_expr()?;

                match (lhs, rhs) {
                    (Expr::Value(lhs), Expr::Value(rhs)) => {
                        let location = lhs.span().start..rhs.span().end();
                        let value = lhs.try_subtract(&*rhs)?;
                        Ok(Expr::Value(Located::new(value, location.clone())))
                    }
                    _ => Ok(self.clone()),
                }
            }
            Expr::Multiply(lhs, rhs) => {
                let lhs = lhs.simplify_expr()?;
                let rhs = rhs.simplify_expr()?;

                match (lhs, rhs) {
                    (Expr::Value(lhs), Expr::Value(rhs)) => {
                        let location = lhs.span().start..rhs.span().end();
                        let value = lhs.try_multiply(&*rhs)?;
                        Ok(Expr::Value(Located::new(value, location.clone())))
                    }
                    _ => Ok(self.clone()),
                }
            }
            Expr::Divide(lhs, rhs) => {
                let lhs = lhs.simplify_expr()?;
                let rhs = rhs.simplify_expr()?;

                match (lhs, rhs) {
                    (Expr::Value(lhs), Expr::Value(rhs)) => {
                        let location = lhs.span().start..rhs.span().end();
                        let value = lhs.try_divide(&*rhs)?;
                        Ok(Expr::Value(Located::new(value, location.clone())))
                    }
                    _ => Ok(self.clone()),
                }
            }

            Expr::LogicalOr(lhs, rhs) => {
                let lhs_loc = lhs.location();
                let lhs = lhs.simplify_expr()?;

                if let Expr::Value(lhs) = &lhs {
                    //if let Value::Boolean(b) = **lhs {
                    if lhs.is_boolean() {
                        if b {
                            return Ok(Expr::Value(lhs.clone()));
                        }
                    }
                }

                let rhs_loc = rhs.location();
                let rhs = rhs.simplify_expr()?;

                if let Expr::Value(rhs) = &rhs {
                    if let Value::Boolean(b) = **rhs {
                        if b {
                            return Ok(Expr::Value(rhs.clone()));
                        } else {
                            return Ok(Expr::Value(Located::new(
                                Value::Boolean(false),
                                lhs_loc.span().start()..rhs_loc.span().end(),
                            )));
                        }
                    }
                }

                Ok(Expr::LogicalOr(
                    Box::new(Located::new(lhs.clone(), lhs_loc)),
                    Box::new(Located::new(rhs.clone(), rhs_loc)),
                ))
            }
            _ => Ok(self.clone()),
        }
    }
     */
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

/*
impl Value {
    pub fn try_add(&self, other: &Self) -> Result<Self, ValueError> {
        match (self, other) {
            (Self::Integer(lhs), Self::Integer(rhs)) => Ok(Self::Integer(lhs + rhs)),
            (Self::Decimal(lhs), Self::Decimal(rhs)) => Ok(Self::Decimal(lhs + rhs)),
            (Self::Decimal(lhs), Self::Integer(rhs)) => Ok(Self::Decimal(lhs + *rhs as f64)),
            (Self::Integer(lhs), Self::Decimal(rhs)) => Ok(Self::Decimal(*lhs as f64 + rhs)),

            _ => Err(ValueError::NonArithmatic),
        }
    }

    pub fn try_subtract(&self, other: &Self) -> Result<Self, ValueError> {
        match (self, other) {
            (Self::Integer(lhs), Self::Integer(rhs)) => Ok(Self::Integer(lhs - rhs)),
            (Self::Decimal(lhs), Self::Decimal(rhs)) => Ok(Self::Decimal(lhs - rhs)),
            (Self::Decimal(lhs), Self::Integer(rhs)) => Ok(Self::Decimal(lhs - *rhs as f64)),
            (Self::Integer(lhs), Self::Decimal(rhs)) => Ok(Self::Decimal(*lhs as f64 - rhs)),
            _ => Err(ValueError::NonArithmatic),
        }
    }

    pub fn try_multiply(&self, other: &Self) -> Result<Self, ValueError> {
        match (self, other) {
            (Self::Integer(lhs), Self::Integer(rhs)) => Ok(Self::Integer(lhs * rhs)),
            (Self::Decimal(lhs), Self::Decimal(rhs)) => Ok(Self::Decimal(lhs * rhs)),
            (Self::Decimal(lhs), Self::Integer(rhs)) => Ok(Self::Decimal(lhs * *rhs as f64)),
            (Self::Integer(lhs), Self::Decimal(rhs)) => Ok(Self::Decimal(*lhs as f64 * rhs)),
            _ => Err(ValueError::NonArithmatic),
        }
    }

    pub fn try_divide(&self, other: &Self) -> Result<Self, ValueError> {
        match (self, other) {
            (Self::Integer(lhs), Self::Integer(rhs)) => {
                if *rhs == 0 {
                    return Err(ValueError::DivideByZero);
                }
                Ok(Self::Integer(lhs / rhs))
            }
            (Self::Decimal(lhs), Self::Decimal(rhs)) => {
                if *rhs == 0.0 {
                    return Err(ValueError::DivideByZero);
                }
                Ok(Self::Decimal(lhs / rhs))
            }
            (Self::Decimal(lhs), Self::Integer(rhs)) => {
                if *rhs == 0 {
                    return Err(ValueError::DivideByZero);
                }
                Ok(Self::Decimal(lhs / *rhs as f64))
            }
            (Self::Integer(lhs), Self::Decimal(rhs)) => {
                if *rhs == 0.0 {
                    return Err(ValueError::DivideByZero);
                }
                Ok(Self::Decimal(*lhs as f64 / rhs))
            }
            _ => Err(ValueError::NonArithmatic),
        }
    }
}
 */

pub fn op(op: &str) -> impl Parser<ParserInput, &str, Error = ParserError> + Clone {
    just(op).padded()
}

pub fn boolean_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    just("true")
        .padded()
        .map_with_span(|_, span: SourceSpan| {
            Located::new(Expr::Value(Located::new(true.into(), span.clone())), span)
        })
        .or(just("false").padded().map_with_span(|_, span: SourceSpan| {
            Located::new(Expr::Value(Located::new(false.into(), span.clone())), span)
        }))
}

pub fn integer_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    text::int::<char, ParserError>(10)
        .map_with_span(|s: String, span| Located::new(s.parse::<i64>().unwrap().into(), span))
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
                    format!("{}.{}", integral, decimal)
                        .parse::<f64>()
                        .unwrap()
                        .into(),
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
            Located::new(Expr::Value(Located::new(x.into(), span.clone())), span)
        })
}

pub fn self_literal() -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    just("self").padded().map_with_span(|v, span: SourceSpan| {
        Located::new(Expr::SelfLiteral(Location::from(span.clone())), span)
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
                Expr::Accessor(
                    Arc::new(Located::new(
                        Expr::SelfLiteral(primary_location.clone()),
                        primary_location.clone(),
                    )),
                    field_name,
                ),
                primary_location.clone(),
            );

            let field_location = primary_location.span().start()..expr.span().end();

            Located::new(
                Expr::Field(Arc::new(expr_self), Arc::new(expr)),
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
            Located::new(Expr::LogicalOr(Arc::new(lhs), Arc::new(rhs)), span)
        })
}

pub fn logical_and(
    expr: impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Expr>, Error = ParserError> + Clone {
    relational_expr(expr.clone())
        .then(op("&&").then(expr).repeated())
        .foldl(|lhs, (_op, rhs)| {
            let span = lhs.span().start()..rhs.span().end();
            Located::new(Expr::LogicalAnd(Arc::new(lhs), Arc::new(rhs)), span)
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
                Located::new(op(Arc::new(lhs), Arc::new(rhs)), span)
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
            Located::new(op(Arc::new(lhs), Arc::new(rhs)), span)
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
            Located::new(op(Arc::new(lhs), Arc::new(rhs)), span)
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_self() {
        let ty = expr()
            .parse(
                r#"
            self
        "#,
            )
            .unwrap()
            .into_inner();

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
            .into_inner();

        println!("{:?}", ty);
    }

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
            .into_inner();

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
