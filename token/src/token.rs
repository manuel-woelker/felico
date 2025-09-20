use felico_source::file_location::FileLocation;
use std::fmt::{Debug, Display};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum TokenKind {
    Fun,
    Identifier,
    ParenOpen,
    ParenClose,
    BraceOpen,
    BraceClose,
    BracketOpen,
    BracketClose,
    Comma,
    Semicolon,
    Colon,
    Dot,
    String,
    EOF,
}

impl TokenKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenKind::Fun => "Fun",
            TokenKind::Identifier => "Identifier",
            TokenKind::ParenOpen => "ParenOpen",
            TokenKind::ParenClose => "ParenClose",
            TokenKind::BraceOpen => "BraceOpen",
            TokenKind::BraceClose => "BraceClose",
            TokenKind::BracketOpen => "BracketOpen",
            TokenKind::BracketClose => "BracketClose",
            TokenKind::Comma => "Comma",
            TokenKind::Semicolon => "Semicolon",
            TokenKind::Colon => "Colon",
            TokenKind::Dot => "Dot",
            TokenKind::String => "String",
            TokenKind::EOF => "EOF",
        }
    }
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(self.as_str())
    }
}

pub type Lexeme<'source> = &'source str;

#[derive(Debug)]
pub struct Token<'source> {
    pub kind: TokenKind,
    pub lexeme: Lexeme<'source>,
    pub location: FileLocation<'source>,
}

impl<'source> Token<'source> {
    pub fn new(
        token_kind: TokenKind,
        lexeme: Lexeme<'source>,
        location: FileLocation<'source>,
    ) -> Self {
        Self {
            kind: token_kind,
            lexeme,
            location,
        }
    }
}
