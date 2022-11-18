use chumsky::prelude::*;
use chumsky::{Error, Parser, Stream};

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

pub struct PolicyParser {}

pub type Span = std::ops::Range<usize>;
pub type Spanned<T> = (T, Span);

pub type ParserInput = char;
pub type ParserError = Simple<char>;

impl PolicyParser {
    pub fn new() -> Self {
        Self {}
    }

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

    fn atom() -> impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone {
        Self::decimal_expr().or(Self::integer_expr())
    }

    fn op(
        text: &'static str,
    ) -> impl Parser<ParserInput, Spanned<String>, Error = ParserError> + Clone {
        just(text)
            .map_with_span(|v, span| (v.to_string(), span))
            .padded()
    }

    fn expr() -> impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone {
        Self::atom()
    }

    fn relational() -> impl Parser<ParserInput, Spanned<Expr>, Error = ParserError> + Clone {
        Self::expr()
            .then(
                (Self::op(">")
                    .to(Expr::GreaterThan as fn(_, _) -> Expr)
                    .or(Self::op(">=").to(Expr::GreaterThanEqual as fn(_, _) -> Expr))
                    .or(Self::op("<").to(Expr::LessThan as fn(_, _) -> Expr))
                    .or(Self::op("<=").to(Expr::LessThanEqual as fn(_, _) -> Expr))
                    .or(Self::op("!=").to(Expr::Inequal as fn(_, _) -> Expr))
                    .or(Self::op("==").to(Expr::Equal as fn(_, _) -> Expr))
                    .then(Self::expr()))
                .repeated()
                .at_most(1),
            )
            .foldl(|lhs, (op, rhs)| {
                let span = lhs.1.start()..rhs.1.end();
                (op(Box::new(lhs), Box::new(rhs)), span)
            })
            .padded()
    }

    pub fn parse<'a, Iter, S>(&self, stream: S) -> Result<Spanned<Expr>, Vec<ParserError>>
    where
        Self: Sized,
        Iter: Iterator<Item = (ParserInput, <ParserError as Error<ParserInput>>::Span)> + 'a,
        S: Into<Stream<'a, ParserInput, <ParserError as Error<ParserInput>>::Span, Iter>>,
    {
        let parser = Self::relational();

        let parser = parser.padded().then_ignore(end());

        parser.parse(stream)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};

    #[test]
    fn decimal() {
        let parser = PolicyParser::new();
        let src = "\n\n42.8821 == \n\t42 > 23";
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
