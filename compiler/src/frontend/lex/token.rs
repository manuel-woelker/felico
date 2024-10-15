use crate::infra::shared_string::SharedString;
use crate::infra::source_span::SourceSpan;
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
    Arrow,

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
    Struct,

    UnexpectedCharacter,

    EOF,
}

impl Display for TokenType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:?}", self))
    }
}

#[derive(Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub location: SourceSpan,
    pub value: Option<SharedString>,
}

impl Token {
    pub fn lexeme(&self) -> &str {
        if let Some(value) = &self.value {
            value
        } else {
            let location = &self.location;
            &self.location.source_file.source_code()
                [location.start_byte as usize..location.end_byte as usize]
        }
    }

    pub fn is_comparison_operator(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::Greater
                | TokenType::GreaterEqual
                | TokenType::Equal
                | TokenType::BangEqual
                | TokenType::Less
                | TokenType::LessEqual
        )
    }
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

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.token_type {
            TokenType::EOF => f.write_str("End of file"),
            _other => {
                write!(f, "'{}' ({})", self.lexeme(), self.token_type)
            }
        }
    }
}
