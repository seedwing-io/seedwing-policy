use chumsky::Parser;
use chumsky::prelude::*;
use crate::lang::{ParserError, ParserInput, Located, Value, Location, FieldName};

#[derive(Clone, Debug)]
pub enum Expr {
    SelfLiteral,
    True,
    False,

    LessThan(Box<Located<Expr>>, Box<Located<Expr>>),
    LessThanEqual(Box<Located<Expr>>, Box<Located<Expr>>),
    GreaterThan(Box<Located<Expr>>, Box<Located<Expr>>),
    GreaterThanEqual(Box<Located<Expr>>, Box<Located<Expr>>),
    Equal(Box<Located<Expr>>, Box<Located<Expr>>),
    Inequal(Box<Located<Expr>>, Box<Located<Expr>>),

    And(Box<Located<Expr>>, Box<Located<Expr>>),
    Or(Box<Located<Expr>>, Box<Located<Expr>>),

    Value(Value),
    Negative(Box<Located<Expr>>),
    Add(Box<Located<Expr>>, Box<Located<Expr>>),
    Subtract(Box<Located<Expr>>, Box<Located<Expr>>),
    Multiply(Box<Located<Expr>>, Box<Located<Expr>>),
    Divide(Box<Located<Expr>>, Box<Located<Expr>>),

    Field(Located<FieldName>, Box<Located<Expr>>),
    //Type(TypeName),
}

pub fn op(
    text: &'static str,
) -> impl Parser<ParserInput, Located<String>, Error=ParserError> + Clone {
    just(text)
        .padded()
        .map_with_span(|v, span| {
            Located::new(v.into(), span)
        })
}

pub fn integer_expr() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    text::int::<char, ParserError>(10)
        .map(|s: String| Value::Integer(s.parse().unwrap()))
        .padded()
        .map_with_span(|value, span| {
            Located::new(Expr::Value(value), span)
        })
}

pub fn decimal_expr() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    text::int(10)
        .then(
            just('.')
                .then(text::int(10)))
        .padded()
        .map(|(integral, (_dot, decimal)): (String, (char, String))| {
            Value::Decimal(format!("{}.{}", integral, decimal).parse().unwrap())
        })
        .map_with_span(|value, span|
            Located::new(Expr::Value(value), span)
        )
}

pub fn self_literal() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    just("self")
        .map_with_span(|v, span| {
            Located::new(Expr::SelfLiteral, span)
        })
}

pub fn field_name() -> impl Parser<ParserInput, Located<FieldName>, Error=ParserError> + Clone {
    text::ident()
        .padded()
        .map_with_span(|v, span| {
            Located::new(FieldName::new(v), span)
        })
}

pub fn field_expr() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    field_name()
        .then(
            just(":")
                .padded()
                .ignored()
        )
        .then(expr())
        .map_with_span(|((name, _colon, ), expr), span| {
            Located::new(
                Expr::Field(
                    name,
                    Box::new(expr),
                ), span,
            )
        })
}

pub fn atom() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    self_literal()
        .or(decimal_expr())
        .or(integer_expr())
}

pub fn expr() -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    recursive(|expr| {
        logical(expr)
    })
}

pub fn logical(expr: impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
    relational(expr.clone())
        .then(
            (op("&&").to(Expr::And as fn(_, _) -> Expr)
                .or(op("||").to(Expr::Or as fn(_, _) -> Expr))
                .then(expr))
                .repeated()
                .at_most(1),
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.span().start()..rhs.span().end();
            Located::new(op(Box::new(lhs), Box::new(rhs)), span)
        })
        .padded()
}

pub fn relational(expr: impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Expr>, Error=ParserError> + Clone {
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
            let span = lhs.span().start()..rhs.span().end();
            Located::new(op(Box::new(lhs), Box::new(rhs)), span)
        })
        .padded()
}


#[cfg(test)]
mod test {
    use super::*;

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
}