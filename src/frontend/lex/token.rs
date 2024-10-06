use crate::infra::location::Location;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Colon,

    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    Identifier,
    String,
    Number,

    // Keywords.
    And,
    Or,
    Else,
    False,
    Fun,
    For,
    If,
    Return,
    True,
    Let,
    While,

    UnexpectedCharacter,

    EOF,
}

impl Display for TokenType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:?}", self))
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub location: Location,
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.token_type {
            TokenType::EOF => f.write_str("End of file"),
            _other => {
                write!(f, "'{}' ({})", self.lexeme(), self.token_type)
            }
        }
    }
}

impl Token {
    pub fn lexeme(&self) -> &str {
        let location = &self.location;
        &self.location.source_file.source_code()
            [location.start_byte as usize..location.end_byte as usize]
    }
}
