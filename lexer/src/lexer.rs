use felico_base::error::FelicoError;
use felico_base::result::FelicoResult;
use felico_source::file_location::FileLocation;
use felico_source::source_file::SourceFile;
use felico_token::{Token, TokenKind};
use std::str::Chars;

pub struct Lexer<'source> {
    start_position: usize,
    current_position: usize,
    chars: Chars<'source>,
    current_char: char,
    next_char: char,
    source_file: &'source SourceFile,
    at_end: bool,
}

const EOF: char = '␄';

impl<'source> Lexer<'source> {
    pub fn new(source_file: &'source SourceFile) -> Self {
        let mut lexer = Self {
            source_file,
            chars: source_file.content().chars(),
            start_position: 0,
            current_position: 0,
            at_end: false,
            current_char: EOF,
            next_char: EOF,
        };
        // Initialize next_char
        lexer.advance();
        lexer.current_position = 0;
        lexer
    }

    fn advance(&mut self) {
        self.current_char = self.next_char;
        if self.current_char != EOF {
            self.current_position += self.current_char.len_utf8();
        }
        self.next_char = self.chars.next().unwrap_or(EOF);
    }

    pub fn next_token(&mut self) -> FelicoResult<Token<'source>> {
        loop {
            self.start_position = self.current_position;
            self.advance();
            if !self.current_char.is_whitespace() {
                break;
            }
        }
        match self.current_char {
            EOF => self.create_token(TokenKind::EOF),
            '(' => self.create_token(TokenKind::ParenOpen),
            ')' => self.create_token(TokenKind::ParenClose),
            '{' => self.create_token(TokenKind::BraceOpen),
            '}' => self.create_token(TokenKind::BraceClose),
            '[' => self.create_token(TokenKind::BracketOpen),
            ']' => self.create_token(TokenKind::BracketClose),
            ',' => self.create_token(TokenKind::Comma),
            ';' => self.create_token(TokenKind::Semicolon),
            ':' => self.create_token(TokenKind::Colon),
            '.' => self.create_token(TokenKind::Dot),
            '"' => loop {
                self.advance();
                match self.current_char {
                    EOF => return Err(FelicoError::message("Unterminated string")),
                    '"' => {
                        return self.create_token(TokenKind::String);
                    }
                    _ => {}
                }
            },
            'a'..='z' | 'A'..='Z' | '_' => {
                loop {
                    self.advance();
                    if !(self.next_char.is_alphanumeric() || self.next_char == '_') {
                        break;
                    }
                }
                let identifier =
                    &self.source_file.content()[self.start_position..self.current_position];
                let token_kind = match identifier {
                    "fun" => TokenKind::Fun,
                    _ => TokenKind::Identifier,
                };
                self.create_token(token_kind)
            }
            other => Err(FelicoError::message(format!(
                "Unexpected character: {other}"
            ))),
        }
    }

    pub fn create_token(&mut self, token_kind: TokenKind) -> FelicoResult<Token<'source>> {
        let location =
            FileLocation::new(self.source_file, self.start_position, self.current_position);
        self.start_position = self.current_position;
        Ok(Token::new(
            token_kind,
            &self.source_file.content()[location.start..location.end],
            location,
        ))
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = FelicoResult<Token<'source>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.at_end {
            None
        } else {
            Some(self.next_token())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use expect_test::{Expect, expect};
    use felico_source::source_file::SourceFile;
    use felico_token::TokenKind;
    use std::fmt::Write;

    fn input_to_test_string(input: &str) -> String {
        let source_file = SourceFile::new("test".to_string(), input.to_string());
        let mut lexer = Lexer::new(&source_file);
        let mut test_string = String::new();
        loop {
            let token = lexer.next_token().unwrap();
            writeln!(
                test_string,
                "🧩 {:3}+{:<2} {:14} {}",
                token.location.start,
                token.location.end - token.location.start,
                token.kind,
                token.lexeme,
            )
            .unwrap();
            if token.kind == TokenKind::EOF {
                break;
            }
        }
        test_string
    }

    fn test_lexer(input: &str, expected: Expect) {
        let test_string = input_to_test_string(input);
        expected.assert_eq(&test_string);
    }

    fn test_lex_symbol(input: &str, expected: &str) {
        let test_string = input_to_test_string(input);
        assert_eq!(
            test_string,
            format!("🧩   0+1  {expected:14} {input}\n🧩   1+0  EOF            \n")
        );
    }

    macro_rules! test_lex_symbol {
        ($(($name:ident $input:literal $expected:literal))*) => {
            $(
            #[test]
            fn $name() {
                test_lex_symbol($input, $expected);
            }
            )*
        };
    }

    test_lex_symbol!(
        (paren_open "(" "ParenOpen")
        (paren_close ")" "ParenClose")
        (brace_open "{" "BraceOpen")
        (brace_close "}" "BraceClose")
        (bracket_open "[" "BracketOpen")
        (bracket_close "]" "BracketClose")
        (comma "," "Comma")
        (semicolon ";" "Semicolon")
        (colon ":" "Colon")
        (dot "." "Dot")
    );

    macro_rules! test_lex {
        ($name:ident, $input:literal, $expected:expr) => {
            #[test]
            fn $name() {
                test_lexer($input, $expected);
            }
        };
    }

    test_lex!(
        empty,
        "",
        expect!([r#"
            🧩   0+0  EOF            
        "#])
    );

    test_lex!(
        parens,
        "()",
        expect!([r#"
            🧩   0+1  ParenOpen      (
            🧩   1+1  ParenClose     )
            🧩   2+0  EOF            
        "#])
    );

    test_lex!(
        string_empty,
        "\"\"",
        expect!([r#"
            🧩   0+2  String         ""
            🧩   2+0  EOF            
        "#])
    );

    test_lex!(
        string_one_char,
        "\"x\"",
        expect!([r#"
            🧩   0+3  String         "x"
            🧩   3+0  EOF            
        "#])
    );

    test_lex!(
        string_multiple_chars,
        "\"hello\"",
        expect!([r#"
            🧩   0+7  String         "hello"
            🧩   7+0  EOF            
        "#])
    );

    test_lex!(
        string_astronaut,
        "\"👨‍🚀\"",
        expect!([r#"
            🧩   0+13 String         "👨‍🚀"
            🧩  13+0  EOF            
        "#])
    );

    test_lex!(
        fun,
        "fun ",
        expect!([r#"
            🧩   0+3  Fun            fun
            🧩   4+0  EOF            
        "#])
    );

    test_lex!(
        identifier,
        "foobar",
        expect!([r#"
            🧩   0+6  Identifier     foobar
            🧩   6+0  EOF            
        "#])
    );

    test_lex!(
        function,
        "fun foo() {}",
        expect!([r#"
            🧩   0+3  Fun            fun
            🧩   4+3  Identifier     foo
            🧩   7+1  ParenOpen      (
            🧩   8+1  ParenClose     )
            🧩  10+1  BraceOpen      {
            🧩  11+1  BraceClose     }
            🧩  12+0  EOF            
        "#])
    );
}
