use chumsky::prelude::*;
use chumsky::{Error, Parser, Stream};

pub type Span = std::ops::Range<usize>;
pub type Spanned<T> = (T, Span);

#[derive(Clone, Debug)]
pub enum Expr {
    This,
    LessThan(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    LessThanEqual(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    GreaterThan(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    GreaterThanEqual(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Equal(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Inequal(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Value(Value),
    Negative(Box<Spanned<Expr>>),
    Add(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Subtract(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Multiply(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    Divide(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
}

#[derive(Clone, Debug)]
pub enum Value {
    Integer(i64),
    Decimal(f64),
    String(String),
    Boolean(bool),
}

#[derive(Clone, Debug)]
pub struct Type {
    name: String,
    fields: Vec<Field>,
}

impl Type {
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: vec![],
        }
    }

    pub fn add_field(&mut self, field: Field) -> &Self {
        self.fields.push(field);
        self
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    name: String,
    expr: Option<Expr>,
}

impl Field {
    pub fn new(name: String, expr: Expr) -> Self {
        Self {
            name,
            expr: Some(expr),
        }
    }
}

pub type ParserInput = char;
pub type ParserError = Simple<char>;

fn integer_expr() -> impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone {
    let value = text::int::<char, ParserError>(10)
        .map(|s: String| Value::Integer(s.parse().unwrap()))
        .padded();

    value.map_with_span(|value, span| (Expr::Value(value), span))
}

fn decimal_expr() -> impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone {
    let value = text::int(10)
        .then(just('.').then(text::int(10)))
        .map(|(integral, (_dot, decimal)): (String, (char, String))| {
            Value::Decimal(format!("{}.{}", integral, decimal).parse().unwrap())
        })
        .padded();

    value.map_with_span(|value, span| (Expr::Value(value), span))
}

fn this() -> impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone {
    just("this").map_with_span(|_, span| (Expr::This, span))
}

fn atom() -> impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone {
    this().or(decimal_expr()).or(integer_expr())
}

fn op(
    text: &'static str,
) -> impl Parser<ParserInput, Spanned<String>, Error = ParserError> + Clone {
    just(text)
        .map_with_span(|v, span| (v.to_string(), span))
        .padded()
}

fn expr() -> impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone {
    recursive(|expr| relational(expr))
}

fn relational(
    expr: impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone {
    atom()
        .then(
            (op(">")
                .to(Expr::GreaterThan as fn(_, _) -> Expr)
                .or(op(">=").to(Expr::GreaterThanEqual as fn(_, _) -> Expr))
                .or(op("<").to(Expr::LessThan as fn(_, _) -> Expr))
                .or(op("<=").to(Expr::LessThanEqual as fn(_, _) -> Expr))
                .or(op("!=").to(Expr::Inequal as fn(_, _) -> Expr))
                .or(op("==").to(Expr::Equal as fn(_, _) -> Expr))
                .then(expr.clone()))
            .repeated()
            .at_most(1),
        )
        .foldl(|lhs, (op, rhs)| {
            let span = lhs.1.start()..rhs.1.end();
            (op(Box::new(lhs), Box::new(rhs)), span)
        })
        .padded()
}

fn type_name() -> impl Parser<ParserInput, Spanned<String>, Error = ParserError> + Clone {
    filter(|c: &char| (c.is_ascii_alphabetic() && c.is_uppercase()) || *c == '_')
        .map(Some)
        .chain::<char, Vec<_>, _>(
            filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_').repeated(),
        )
        .collect()
        .map_with_span(|v, span| (v, span))
}

fn ty() -> impl Parser<ParserInput, Spanned<Type>, Error = ParserError> + Clone {
    just("type")
        .padded()
        .map_with_span(|_, span| span)
        .then(
            type_name()
                .padded()
                .map(|(name, _span)| Type::new(name))
                .then(just("{").padded().ignored())
                .then(
                    field()
                        .separated_by(just(",").padded().ignored())
                        .allow_trailing(),
                )
                .then(just("}").padded().map_with_span(|_args, span| span))
                .map(|((mut ty, fields), end_span)| {
                    for f in fields {
                        ty.0.add_field(f.0);
                    }
                    (ty, end_span)
                }),
        )
        .map(|(start_span, ((ty, _), end_span))| (ty, start_span.start()..end_span.end()))
}

fn field() -> impl Parser<ParserInput, Spanned<Field>, Error = ParserError> + Clone {
    text::ident()
        .padded()
        .map_with_span(|v, span| (v, span))
        .then(just(":").padded().ignored())
        .then(expr())
        .map(|(((name, span), _), expr)| (Field::new(name, expr.0), span.start()..expr.1.end()))
}

#[derive(Copy, Clone, Default)]
pub struct PolicyParser {}

impl PolicyParser {
    pub fn parse<'a, Iter, S>(&self, stream: S) -> Result<Spanned<Expr>, Vec<ParserError>>
    where
        Self: Sized,
        Iter: Iterator<Item = (ParserInput, <ParserError as Error<ParserInput>>::Span)> + 'a,
        S: Into<Stream<'a, ParserInput, <ParserError as Error<ParserInput>>::Span, Iter>>,
    {
        let parser = expr();
        let parser = parser.padded().then_ignore(end());

        parser.parse(stream)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};

    #[test]
    fn parse_type_name() {
        let result = type_name().parse("Bob").unwrap();
        assert_eq!("Bob", result.0);

        let result = type_name().parse("bob");
        assert!(matches!(result, Err(_)));
    }

    #[test]
    fn parse_type() {
        let result = ty().parse(
            r#"
        type Bob {
            age: this > 49,
            name: this < 23,
        }
        "#,
        );

        println!("{:?}", result);
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
