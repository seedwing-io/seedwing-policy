use std::fmt::{Debug, Display, Formatter};
use std::io::{BufReader, Read, Seek};
use miette::SourceSpan;

#[derive(Debug, Clone)]
pub struct Token {
    ty: TokenType,
    span: SourceSpan,
}

impl Token {
    pub fn new(ty: TokenType, span: SourceSpan) -> Self {
        Self {
            ty,
            span
        }
    }

    pub fn ty(&self) -> &TokenType {
        &self.ty
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }
}

impl From<Token> for SourceSpan {
    fn from(t: Token) -> Self {
        t.span
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt( &self.ty, f)
    }
}

#[derive(Debug, Clone)]
pub enum TokenType {
    Colon,
    Semicolon,
    Dot,
    Comma,
    And,
    Or,
    Identifier(String),
    Integer(i64),
    Decimal(f64),
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    Equal,
    LeftCurlyBrace,
    RightCurlyBrace,
    LeftSquareBracket,
    RightSquareBracket,
    LeftParenthesis,
    RightParenthesis,
    //
    Nl,
    Crnl,
}

impl TokenType {
    pub fn len(&self) -> usize {
        match self {
            TokenType::Colon => 1,
            TokenType::Semicolon => 1,
            TokenType::Dot => 1,
            TokenType::Comma => 1,
            TokenType::And => 2,
            TokenType::Or => 2,
            TokenType::Identifier(id) => id.len(),
            TokenType::Integer(val) => {
                format!("{}", val).len()
            }
            TokenType::Decimal(val) => {
                format!("{}", val).len()
            }
            TokenType::LessThan => 1,
            TokenType::LessThanEqual => 2,
            TokenType::GreaterThan => 1,
            TokenType::GreaterThanEqual => 2,
            TokenType::Equal => 2,
            TokenType::LeftCurlyBrace => 1,
            TokenType::RightCurlyBrace => 1,
            TokenType::LeftSquareBracket => 1,
            TokenType::RightSquareBracket => 1,
            TokenType::LeftParenthesis => 1,
            TokenType::RightParenthesis => 1,
            TokenType::Nl => 1,
            TokenType::Crnl => 2,
        }

    }
}

impl Display for TokenType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenType::Colon => todo!(),
            TokenType::Semicolon => todo!(),
            TokenType::Dot => todo!(),
            TokenType::Comma => todo!(),
            TokenType::And => todo!(),
            TokenType::Or => todo!(),
            TokenType::Identifier(_) => todo!(),
            TokenType::Integer(value) => write!(f, "an integer: {}", value),
            TokenType::Decimal(_) => todo!(),
            TokenType::LessThan => write!(f, "'<'"),
            TokenType::LessThanEqual => write!(f, "'<='"),
            TokenType::GreaterThan => write!(f, "'>'"),
            TokenType::GreaterThanEqual => write!(f, "'>='"),
            TokenType::Equal => todo!(),
            TokenType::LeftCurlyBrace => todo!(),
            TokenType::RightCurlyBrace => todo!(),
            TokenType::LeftSquareBracket => todo!(),
            TokenType::RightSquareBracket => todo!(),
            TokenType::LeftParenthesis => write!(f, "'('"),
            TokenType::RightParenthesis => write!(f, "')'"),
            TokenType::Nl => write!(f, "\\n"),
            TokenType::Crnl => write!(f, "\\r\\n"),
        }
    }
}

