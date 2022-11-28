use chumsky::Parser;
use chumsky::prelude::*;
use crate::lang::{ParserError, ParserInput, Located, Location, FieldName, ComparisonOp, DerivationOp};
use crate::lang::ty::Type;

#[derive(Debug, Clone)]
pub enum Expr {
    SelfLiteral,
    /* self */
    Value(Located<Value>),
    Navigation(Box<Located<Expr>>, Located<String>),
    /* self.len */
    Function(Located<String>, Box<Located<Expr>>),
    /* len(self) */
    Add(Box<Located<Expr>>, Box<Located<Expr>>),
    Subtract(Box<Located<Expr>>, Box<Located<Expr>>),
    Multiply(Box<Located<Expr>>, Box<Located<Expr>>),
    Divide(Box<Located<Expr>>, Box<Located<Expr>>),
    LessThan(Box<Located<Expr>>, Box<Located<Expr>>),
    LessThanEqual(Box<Located<Expr>>, Box<Located<Expr>>),
    GreaterThan(Box<Located<Expr>>, Box<Located<Expr>>),
    GreaterThanEqual(Box<Located<Expr>>, Box<Located<Expr>>),
    Equal(Box<Located<Expr>>, Box<Located<Expr>>),
    NotEqual(Box<Located<Expr>>, Box<Located<Expr>>),
    LogicalAnd(Box<Located<Expr>>, Box<Located<Expr>>),
    LogicalOr(Box<Located<Expr>>, Box<Located<Expr>>),
}

#[derive(Clone, Debug)]
pub enum Value {
    Integer(i64),
    Decimal(f64),
    String(String),
    Boolean(bool),
}

pub fn op(op: &str) -> impl Parser<ParserInput, &str, Error=ParserError> + Clone {
    just(op).padded()
}

pub fn integer_expr() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    text::int::<char, ParserError>(10)
        .map_with_span(|s: String, span| {
            Located::new(
                Value::Integer(s.parse().unwrap()),
                span,
            )
        })
        .padded()
        .map_with_span(|value, span| {
            Located::new(
                Expr::Value(value),
                span,
            )
        })
}

pub fn decimal_expr() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    text::int(10)
        .then(
            just('.')
                .then(text::int(10)))
        .padded()
        .map_with_span(|(integral, (_dot, decimal)): (String, (char, String)), span| {
            Located::new(
                Value::Decimal(format!("{}.{}", integral, decimal).parse().unwrap()),
                span)
        })
        .map_with_span(|value, span|
            Located::new(
                Expr::Value(value),
                span,
            )
        )
}

pub fn self_literal() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    just("self")
        .padded()
        .map_with_span(|v, span| {
            Located::new(
                Expr::SelfLiteral,
                span)
        })
}

pub fn atom() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    self_literal()
        .or(decimal_expr())
        .or(integer_expr())
}

pub fn expr() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    recursive(|expr| {
        parenthesized_expr(expr.clone())
            .or(
                logical_or(expr)
            )
    }).then_ignore(end())
}

pub fn parenthesized_expr(expr: impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    just("(").padded().ignored()
        .then(expr)
        .then(just(")").padded().ignored())
        .map(|((_left_paren, expr), _right_paren)| {
            expr
        })
}

pub fn logical_or(expr: impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    logical_and(expr.clone())
        .then(
            op("||").then(expr.clone()).repeated()
        )
        .foldl(|lhs, (_op, rhs)| {
            let span = lhs.span().start()..rhs.span().end();
            Located::new(
                Expr::LogicalOr(
                    Box::new(lhs),
                    Box::new(rhs)), span)
        })
}

pub fn logical_and(expr: impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone)
                   -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    relational_expr(expr.clone())
        .then(
            op("&&").then(expr.clone()).repeated()
        )
        .foldl(|lhs, (_op, rhs)| {
            let span = lhs.span().start()..rhs.span().end();
            Located::new(
                Expr::LogicalAnd(
                    Box::new(lhs),
                    Box::new(rhs)), span)
        })
}

pub fn relational_expr(expr: impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone)
                       -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    additive_expr(expr.clone())
        .then(
            op(">=").map_with_span(|_, span| Located::new(Expr::GreaterThanEqual as fn(_, _) -> _, span))
                .or(op(">").map_with_span(|_, span| Located::new(Expr::GreaterThan as fn(_, _) -> _, span)))
                .or(op("<=").map_with_span(|_, span| Located::new(Expr::LessThanEqual as fn(_, _) -> _, span)))
                .or(op("<").map_with_span(|_, span| Located::new(Expr::LessThan as fn(_, _) -> _, span)))
                .or(op("==").map_with_span(|_, span| Located::new(Expr::Equal as fn(_, _) -> _, span)))
                .or(op("!=").map_with_span(|_, span| Located::new(Expr::NotEqual as fn(_, _) -> _, span)))
                .then(expr).or_not()
        ).map(|(lhs, rhs)| {
        if let Some((op, rhs)) = rhs {
            let span = op.span().start()..rhs.span().end;
            Located::new(
                op(Box::new(lhs), Box::new(rhs)),
                span)
        } else {
            lhs
        }
    })
}


pub fn additive_expr(expr: impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone)
                     -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    multiplicative_expr(expr.clone())
        .then(
            op("+").map_with_span(|_, span| Located::new(Expr::Add as fn(_, _) -> _, span))
                .or(op("-").map_with_span(|_, span| Located::new(Expr::Subtract as fn(_, _) -> _, span)))
                .then(multiplicative_expr(expr.clone())).repeated()
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.span().start()..rhs.span().end;
            Located::new(
                op(Box::new(lhs),
                   Box::new(rhs)),
                span)
        })
}

pub fn multiplicative_expr(expr: impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone)
                           -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    atom()
        .then(
            op("*").map_with_span(|_, span| Located::new(Expr::Multiply as fn(_, _) -> _, span))
                .or(op("/").map_with_span(|_, span| Located::new(Expr::Divide as fn(_, _) -> _, span)))
                .then(atom()).repeated()
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.span().start()..rhs.span().end;
            Located::new(
                op(Box::new(lhs),
                   Box::new(rhs)),
                span)
        })
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_self() {
        let ty = expr().parse(r#"
            self
        "#).unwrap().into_inner();

        assert!(matches!( ty, Expr::SelfLiteral ));
    }

    #[test]
    fn parse_integer_literal() {
        let ty = expr().parse(r#"
            42
        "#).unwrap().into_inner();

        assert!(matches!( ty,
            Expr::Value(
                Located {
                    inner: Value::Integer(42),
                    ..
                }
        )));
    }

    #[test]
    fn parse_decimal_literal() {
        let ty = expr().parse(r#"
            42.1415
        "#).unwrap().into_inner();

        assert!(matches!( ty,
            Expr::Value(
                Located {
                    inner:  Value::Decimal(x),
                    ..
                } )
            if x > 42.1 && x < 42.2) );
    }

    #[test]
    fn parse_parenthesized_expr() {
        let ty = expr().parse(r#"
            (42 + 88)
        "#).unwrap().into_inner();

        println!("{:?}", ty);
    }

    #[test]
    fn parse_math() {
        let ty = expr().parse(r#"
            self * 1 + 2 * 3 + 4
        "#).unwrap().into_inner();

        println!("{:?}", ty);
    }

    /*
    #[test]
    fn parse_field_expr() {
        let expr = field_expr().parse(r#"
            foo: 42
        "#).unwrap();

        let expr = expr.into_inner();

        assert!(
            matches!(
                expr,
                Expr::Field(..)
            )
        );

        println!("{:?}", expr);

        if let Expr::Field(name, expr) = expr {
            assert_eq!("foo", name.name());
        }
    }

     */
}