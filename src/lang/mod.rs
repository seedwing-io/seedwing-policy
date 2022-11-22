use chumsky::prelude::*;
use chumsky::{Error, Parser, Stream};
use crate::lang::expression::{Expr, expr};

mod expression;
mod ty;

pub type Span = std::ops::Range<usize>;
pub type Spanned<T> = (T, Span);

#[derive(Clone, Debug)]
pub enum Value {
    Integer(i64),
    Decimal(f64),
    String(String),
    Boolean(bool),
}



#[allow(unused)]
type ParserInput = char;
#[allow(unused)]
type ParserError = Simple<char>;


#[derive(Copy, Clone, Default)]
pub struct PolicyParser {}

impl PolicyParser {
    pub fn parse<'a, Iter, S>(&self, stream: S) -> Result<Spanned<Expr>, Vec<ParserError>>
        where
            Self: Sized,
            Iter: Iterator<Item=(ParserInput, <ParserError as Error<ParserInput>>::Span)> + 'a,
            S: Into<Stream<'a, ParserInput, <ParserError as Error<ParserInput>>::Span, Iter>>,
    {
        let parser = expr();
        let parser = parser.padded().then_ignore(end());

        parser.parse(stream)
    }
}

/*
#[cfg(test)]
mod test {
    use super::*;
    use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
    use crate::lang::ty::type_name;

    #[test]
    fn parse_logical() {
        let result = expr().parse(r#"
            Tall && Tired
        "#);

        println!("{:?}", result);
    }

    #[test]
    fn parse_type_name() {
        let result = type_name().parse("Bob").unwrap();
        assert_eq!("Bob", result.0.0);

        let result = type_name().parse("bob");
        assert!(matches!( result, Err(_)));
    }

    #[test]
    fn parse_expr_type() {
        let result = ty().parse(r#"
            type Bob := Tall && DogOwner;
        "#).unwrap();

        assert_eq!("Bob", result.0.name);
    }

    #[test]
    fn parse_object_type() {
        let result = ty().parse(r#"
        type Bob := {
            age: this > 49,
            name: this < 23,
        }
        "#).unwrap();

        println!("{:?}", result);

        assert_eq!("Bob", result.0.name);
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
 */