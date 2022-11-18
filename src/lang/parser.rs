use crate::lang::lexer::token::{Token, TokenType};
use crate::lang::lexer::{Error, Lexer, TokenStream};
use crate::lang::Constraint;
use crate::{DecimalType, IntegerType, Type};
use miette::{Diagnostic, NamedSource, SourceCode, SourceSpan};
use std::fmt::{Display, Formatter};
use std::io::{BufReader, Read, Seek, SeekFrom};
use thiserror::Error;

#[derive(Debug)]
struct ExpectedTokens(pub Vec<TokenType>);

impl Display for ExpectedTokens {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = if self.0.len() < 5 {
            let bits: Vec<String> = self.0.iter().map(|e| format!("{}", e)).collect();
            bits.join(", ")
        } else {
            "A lot of tokens".into()
        };

        write!(f, "{}", text)
    }
}

#[derive(Diagnostic, Error, Debug)]
pub enum ParseError {
    #[error(transparent)]
    Lexer(Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    UnexpectedToken(UnexpectedToken),

    #[error("Unknown error")]
    Unknown,
}

impl ParseError {
    pub fn unknown() -> Self {
        ParseError::Unknown
    }
}

#[derive(Diagnostic, Error, Debug)]
#[error("Expected {} but instead saw {}", .expected, .token)]
#[diagnostic(code(dogma::unexpected_token))]
pub struct UnexpectedToken {
    #[source_code]
    src: String,

    expected: ExpectedTokens,
    token: Token,
    #[label("The token")]
    highlight: SourceSpan,
}

impl From<Error> for ParseError {
    fn from(inner: Error) -> Self {
        ParseError::Lexer(inner)
    }
}

impl From<UnexpectedToken> for ParseError {
    fn from(inner: UnexpectedToken) -> Self {
        ParseError::UnexpectedToken(inner)
    }
}
pub struct Parser<R: Read + Seek> {
    input: TokenStream<R>,
}

impl<R: Read + Seek> Parser<R> {
    pub fn new(input: R) -> Self {
        Self {
            input: TokenStream::new(input),
        }
    }

    fn unexpected_token(&mut self, token: Token, expected: Vec<TokenType>) -> UnexpectedToken {
        UnexpectedToken {
            src: self.source(),
            highlight: token.span(),
            expected: ExpectedTokens(expected),
            token,
        }
    }

    fn source(&mut self) -> String {
        let mut source = self.input.source_mut();
        let pos = source.stream_position().unwrap_or(0);
        source.seek(SeekFrom::Start(0));
        let mut code = String::new();
        source.read_to_string(&mut code);
        source.seek(SeekFrom::Start(pos));
        println!("CODE [{}]", code);
        code
    }

    pub fn ty(&mut self) -> Result<Type, ParseError> {
        println!("ty()");
        let mut ty;

        let t = self.input.la(1)?;
        println!("--> {:?}", t);
        ty = match t.ty() {
            TokenType::LessThan => self.less_than()?,
            TokenType::LessThanEqual => self.less_than_equal()?,
            TokenType::GreaterThan => self.greater_than()?,
            TokenType::GreaterThanEqual => self.greater_than_equal()?,
            _ => {
                return Err(self
                    .unexpected_token(
                        t,
                        vec![
                            TokenType::LessThan,
                            TokenType::LessThanEqual,
                            TokenType::GreaterThan,
                            TokenType::GreaterThanEqual,
                        ],
                    )
                    .into());
            }
        };

        loop {
            let t = self.input.la(1);

            match t {
                Err(Error::EndOfSource) => {
                    break;
                }
                Ok(t) => match t.ty() {
                    TokenType::And => {
                        println!("ty AND>");
                        ty = self.and(ty)?;
                        println!("ty AND<");
                    }
                    TokenType::Or => {
                        println!("ty OR>");
                        ty = self.or(ty)?;
                        println!("ty OR<");
                    }
                    _ => {
                        return Err(self
                            .unexpected_token(t, vec![TokenType::And, TokenType::Or])
                            .into())
                    }
                },
                Err(inner) => return Err(inner.into()),
            }
        }

        Ok(ty)
    }

    pub fn and(&mut self, lhs: Type) -> Result<Type, ParseError> {
        println!("and()");
        let and = self.input.consume(1)?;
        if let TokenType::And = and.ty() {
            let rhs = self.ty()?;
            Ok(lhs.join(&rhs))
        } else {
            Err(ParseError::unknown())
        }
    }

    pub fn or(&mut self, lhs: Type) -> Result<Type, ParseError> {
        println!("or()");
        let or = self.input.consume(1)?;
        if let TokenType::Or = or.ty() {
            let rhs = self.ty()?;
            Ok(lhs.join(&rhs))
        } else {
            Err(ParseError::unknown())
        }
    }

    pub fn less_than(&mut self) -> Result<Type, ParseError> {
        println!("lt()");
        let lt = self.input.consume(1)?;
        println!("lt() a");

        if let TokenType::LessThan = lt.ty() {
            println!("lt() b");
            let rh = self.input.consume(1)?;
            println!("lt() c");

            match rh.ty() {
                TokenType::Integer(num) => Ok(IntegerType::less_than(*num)),
                TokenType::Decimal(num) => Ok(DecimalType::LessThan(*num).into()),
                _ => Err(ParseError::unknown()),
            }
        } else {
            Err(ParseError::unknown())
        }
    }

    pub fn less_than_equal(&mut self) -> Result<Type, ParseError> {
        println!("lte()");
        let lte = self.input.consume(1)?;

        if let TokenType::LessThanEqual = lte.ty() {
            let rh = self.input.consume(1)?;

            match rh.ty() {
                TokenType::Integer(num) => Ok(IntegerType::less_than_or_equal(*num)),
                TokenType::Decimal(num) => {
                    //Ok(DecimalType::greater_than_equal(*num))
                    //Ok(DecimalType::GreaterThan(*num).into())
                    todo!()
                }
                _ => Err(ParseError::unknown()),
            }
        } else {
            Err(ParseError::unknown())
        }
    }

    pub fn greater_than(&mut self) -> Result<Type, ParseError> {
        println!("gt()");
        let gt = self.input.consume(1)?;

        if let TokenType::GreaterThan = gt.ty() {
            let rh = self.input.consume(1)?;

            match rh.ty() {
                TokenType::Integer(num) => Ok(IntegerType::greater_than(*num)),
                TokenType::Decimal(num) => Ok(DecimalType::GreaterThan(*num).into()),
                _ => Err(ParseError::unknown()),
            }
        } else {
            Err(ParseError::unknown())
        }
    }

    pub fn greater_than_equal(&mut self) -> Result<Type, ParseError> {
        println!("gte()");
        let gte = self.input.consume(1)?;

        if let TokenType::GreaterThanEqual = gte.ty() {
            let rh = self.input.consume(1)?;

            match rh.ty() {
                TokenType::Integer(num) => Ok(IntegerType::greater_than_equal(*num)),
                TokenType::Decimal(num) => {
                    //Ok(DecimalType::greater_than_equal(*num))
                    //Ok(DecimalType::GreaterThan(*num).into())
                    todo!()
                }
                _ => Err(ParseError::unknown()),
            }
        } else {
            Err(ParseError::unknown())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::lexer::token::{Token, TokenType};
    use crate::Type;
    use miette::{
        GraphicalReportHandler, GraphicalTheme, IntoDiagnostic, MietteHandler, NamedSource, Report,
        SourceSpan,
    };
    use std::io::Cursor;
    use std::str::from_utf8;

    #[test]
    fn simple_parse() {
        let source = Cursor::new(">= 42 || < 12");
        let mut parser = Parser::new(source);

        let ty = parser.ty().unwrap();

        assert!(ty.accepts(Type::integer(11)));
        assert!(!ty.accepts(Type::integer(12)));
        assert!(ty.accepts(Type::integer(42)));
        assert!(!ty.accepts(Type::integer(41)));
    }

    #[test]
    fn invalid_parse() {
        let source = Cursor::new("\n\n\n42 what howdy\nyeah");
        let mut parser = Parser::new(source);

        let result = parser.ty();


        if let Err(diag) = result {

            let diag: Report = diag.into();

            let mut out = String::new();
            GraphicalReportHandler::new_themed(GraphicalTheme::unicode())
                .with_width(80)
                .render_report(&mut out, diag.as_ref())
                .unwrap();

            println!("------------------------------------");
            println!("{}", out);
            println!("------------------------------------");
        }
    }
}
