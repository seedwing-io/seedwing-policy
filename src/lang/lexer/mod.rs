use std::collections::LinkedList;
use std::fmt::{Display, Formatter};
use std::io::{BufReader, Read, Seek};
use crate::lang::lexer::token::{Token, TokenType};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

pub mod token;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unknown")]
    Unknown,
    #[error("End of source")]
    EndOfSource,
    #[error("UTF-8")]
    Utf8,
    #[error("Invalid token")]
    InvalidToken,
}

pub struct TokenStream<R: Read + Seek> {
    lexer: Lexer<R>,
    buffer: LinkedList<Token>,
}

impl<R: Read + Seek> TokenStream<R> {
    pub fn new(input: R) -> Self {
        Self {
            lexer: Lexer::new(input),
            buffer: Default::default()
        }
    }

    pub(crate) fn source_mut(&mut self) -> &mut BufReader<R> {
        self.lexer.source_mut()
    }

    pub fn la(&mut self, offset: usize) -> Result<Token, Error> {
        while self.buffer.len() < offset {
            let token = self.lexer.next_token()?;
            self.buffer.push_back( token )
        }

        let token = self.buffer.iter().nth(offset-1);
        if let Some(token) = token {
            Ok(token.clone())
        } else {
            Err(Error::EndOfSource)
        }
    }


    pub fn consume(&mut self, amount: usize) -> Result<Token,Error> {
        while self.buffer.len() < amount {
            let token = self.lexer.next_token()?;
            self.buffer.push_back( token )
        }

        let mut token = Err(Error::EndOfSource);

        for _ in 0..amount {
            if let Some(cur) = self.buffer.pop_front() {
                token = Ok(cur)
            } else {
                return Err(Error::EndOfSource)
            }
        }
        token
    }
}

pub struct Lexer<R: Read + Seek> {
    input: BufReader<R>,
    cur: usize,
}

impl<R: Read + Seek> Lexer<R> {
    pub fn new(input: R) -> Self {
        Self {
            input: BufReader::new(input),
            cur: 0,
        }
    }

    pub(crate) fn source_mut(&mut self) -> &mut BufReader<R> {
        &mut self.input
    }

    fn token(ty: TokenType, cur: usize) -> Token {
        let len = ty.len();
        Token::new(ty, SourceSpan::new(cur.into(), len.into()))
    }

    fn la(&mut self, offset: i64) -> Result<u8, Error> {
        let mut la = 0;
        for _ in 0..offset {
            la = self.read()?;
        }
        self.input.seek_relative(-1 * offset).ok();
        Ok(la)
    }

    fn consume(&mut self, amount: usize) -> Result<u8, Error> {
        let mut c = 0;
        for _ in 0..amount {
            c = self.read()?;
            self.cur += 1;
        }

        Ok(c)
    }

    fn read(&mut self) -> Result<u8, Error> {
        let mut buf = [0u8; 1];
        if let Ok(size) = self.input.read(&mut buf) {
            if size > 0 {
                Ok(buf[0])
            } else {
                Err(Error::EndOfSource)
            }
        } else {
            Err(Error::Unknown)
        }
    }

    fn cur(&self) -> usize {
        self.cur
    }

    pub fn next_token(&mut self) -> Result<Token, Error> {
        loop {
            let c = self.la(1)?;

            return match c {
                b':' => self.colon(),
                b';' => self.semicolon(),
                b',' => self.dot(),
                b',' => self.comma(),
                b'&' => self.and(),
                b'|' => self.or(),
                b'{' => self.left_curly_brace(),
                b'}' => self.right_curly_brace(),
                b'[' => self.left_square_bracket(),
                b']' => self.right_square_bracket(),
                b'(' => self.left_parenthesis(),
                b')' => self.right_parenthesis(),
                b'<' => {
                    if self.la(2)? == b'=' {
                        self.less_than_equal()
                    } else {
                        self.less_than()
                    }
                }
                b'>' => {
                    if self.la(2)? == b'=' {
                        self.greater_than_equal()
                    } else {
                        self.greater_than()
                    }
                }
                b'0'..=b'9' => self.number(),
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.identifier(),
                b'\r' | b'\n' => {
                    self.line_break()?;
                    continue;
                }
                b'\t' | b' ' => {
                    self.whitespace()?;
                    continue;
                }
                _ => {
                    println!("whut {:?}", c as char);
                    Err(Error::InvalidToken)
                }
            };
        }
    }

    fn line_break(&mut self) -> Result<(), Error> {
        let c = self.consume(1)?;
        if c == b'\r' {
            if self.la(1)? == b'\n' {
                self.consume(1);
            }
        }
        Ok(())
    }

    fn whitespace(&mut self) -> Result<(), Error> {
        loop {
            let c = self.consume(1)?;
            let la = self.la(1)?;
            if la == b'\t' || la == b' ' {
                continue;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn colon(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::Colon, cur))
    }

    fn semicolon(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::Semicolon, cur))
    }

    fn dot(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::Dot, cur))
    }

    fn comma(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::Comma, cur))
    }

    fn and(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(2)?;
        Ok(Self::token(TokenType::And, cur))
    }

    fn or(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(2)?;
        Ok(Self::token(TokenType::Or, cur))
    }

    fn left_curly_brace(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::LeftCurlyBrace, cur))
    }

    fn right_curly_brace(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::RightCurlyBrace, cur))
    }

    fn left_square_bracket(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::LeftSquareBracket, cur))
    }

    fn right_square_bracket(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::RightSquareBracket, cur))
    }

    fn left_parenthesis(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::LeftParenthesis, cur))
    }

    fn right_parenthesis(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token( TokenType::RightParenthesis, cur))
    }

    fn less_than(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::LessThan, cur))
    }

    fn less_than_equal(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(2)?;
        Ok(Self::token(TokenType::LessThanEqual, cur))
    }

    fn greater_than(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(1)?;
        Ok(Self::token(TokenType::GreaterThan, cur))
    }

    fn greater_than_equal(&mut self) -> Result<Token, Error> {
        let cur = self.cur;
        self.consume(2)?;
        Ok(Self::token(TokenType::GreaterThanEqual, cur))
    }

    fn number(&mut self) -> Result<Token, Error> {
        println!("number");
        let cur = self.cur;
        let mut accum = vec![];

        loop {
            let c = self.consume(1)?;
            accum.push(c);

            match self.la(1) {
                Ok(b'0'..=b'9' | b'_') => {
                    println!("AA");
                    // okay, continue to next loop-de-loop
                }
                Ok(b'.') => {
                    println!("BB");
                    // it's gone decimal
                    loop {
                        let c = self.consume(1)?;
                        accum.push(c);

                        match self.la(1) {
                            Ok(b'0'..=b'9') => {
                                // okay, conitinue
                            }
                            Ok(_) | Err(Error::EndOfSource) => {
                                let accum = String::from_utf8(accum).map_err(|_|Error::Utf8)?;
                                let result = accum.parse::<f64>().map_err(|_| Error::InvalidToken)?;
                                return Ok(Self::token(
                                    TokenType::Decimal(result),
                                    cur,
                                ));

                            }
                            _ => return Err(Error::Unknown)
                        }
                    }
                }
                Ok(_) | Err(Error::EndOfSource) => {
                    println!("CC");
                    let accum = String::from_utf8(accum).map_err(|_|Error::Utf8)?;
                    let result = accum.parse::<i64>().map_err(|_| Error::InvalidToken)?;
                    return Ok(Self::token(
                        TokenType::Integer(result),
                        cur,
                    ));
                }
                _ => return Err(Error::Unknown)
            }
        }

    }


    fn identifier(&mut self) -> Result<Token, Error> {
        let cur = self.cur();
        let mut accum = vec![];

        loop {
            let c = self.consume(1)?;
            accum.push(c);

            match self.la(1) {
                Ok(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') => {
                    // okay, continue to next loop-de-loop
                }
                Ok(_) | Err(Error::EndOfSource) => {
                    return Ok(Self::token(
                        TokenType::Identifier(String::from_utf8(accum).map_err(|_| Error::Utf8)?),
                        cur,
                    ));
                }
                _ => return Err(Error::Unknown)
            }
        }
    }
}

impl<R: Read + Seek> Iterator for Lexer<R> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(next) = self.next_token() {
            Some(next)
        } else {
            None
        }
    }
}


#[cfg(test)]
mod test {
    use std::io::Cursor;
    use super::*;

    #[test]
    fn simple_tokenizing() {
        let source = Cursor::new("bob,  jim:\n  ([ulf])");
        let mut lexer = Lexer::new(source);

        for token in lexer {
            println!("{:?}", token);
        }
    }
}