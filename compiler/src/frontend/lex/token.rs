use crate::infra::source_span::SourceSpan;
use crate::model::workspace::WorkspaceString;
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
    ColonColon,

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
    Trait,
    Enum,
    Impl,

    UnexpectedCharacter,

    EOF,
}

impl Display for TokenType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:?}", self))
    }
}

#[derive(Copy, Clone)]
pub struct Token<'ws> {
    pub inner: &'ws TokenInner<'ws>,
}

pub struct TokenInner<'ws> {
    pub token_type: TokenType,
    pub location: SourceSpan<'ws>,
    pub value: WorkspaceString<'ws>,
}

impl<'ws> Token<'ws> {
    pub fn lexeme(&self) -> &'ws str {
        self.inner.value
    }

    pub fn is_comparison_operator(&self) -> bool {
        matches!(
            self.inner.token_type,
            TokenType::Greater
                | TokenType::GreaterEqual
                | TokenType::Equal
                | TokenType::BangEqual
                | TokenType::Less
                | TokenType::LessEqual
        )
    }

    pub fn token_type(&self) -> TokenType {
        self.inner.token_type
    }

    pub fn location(&self) -> &SourceSpan<'ws> {
        &self.inner.location
    }
}

impl<'ws> Display for Token<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.inner.token_type {
            TokenType::EOF => f.write_str("End of file"),
            _other => {
                write!(f, "'{}' ({})", self.lexeme(), self.inner.token_type)
            }
        }
    }
}

impl<'ws> Debug for Token<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.inner.token_type {
            TokenType::EOF => f.write_str("End of file"),
            _other => {
                write!(f, "'{}' ({})", self.lexeme(), self.inner.token_type)
            }
        }
    }
}
