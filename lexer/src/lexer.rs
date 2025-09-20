use crate::token::{Token, TokenKind};
use felico_base::error::FelicoError;
use felico_base::result::FelicoResult;
use felico_source::file_location::FileLocation;
use felico_source::source_file::SourceFile;
use std::str::Chars;

pub struct Lexer<'source> {
    start_position: usize,
    current_position: usize,
    chars: Chars<'source>,
    source_file: &'source SourceFile,
}

impl<'source> Lexer<'source> {
    pub fn new(source_file: &'source SourceFile) -> Self {
        Self {
            source_file,
            chars: source_file.content().chars(),
            start_position: 0,
            current_position: 0,
        }
    }

    pub fn next_token(&mut self) -> FelicoResult<Token<'source>> {
        let Some(current_char) = self.chars.next() else {
            return self.create_token(TokenKind::EOF);
        };
        self.current_position += current_char.len_utf8();
        match current_char {
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
                let Some(next_char) = self.chars.next() else {
                    return Err(FelicoError::message("Unterminated string"));
                };
                self.current_position += next_char.len_utf8();
                if next_char == '"' {
                    return self.create_token(TokenKind::String);
                }
            },
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

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::token::TokenKind;
    use expect_test::{Expect, expect};
    use felico_source::source_file::SourceFile;
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
}
