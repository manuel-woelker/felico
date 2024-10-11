use phf::phf_map;
use std::fmt::{Debug, Formatter};
use std::str::Chars;

use crate::frontend::lex::token::{Token, TokenType};
use crate::infra::location::{ByteOffset, Location};
use crate::infra::result::FelicoResult;
use crate::infra::source_file::SourceFile;
use ouroboros::self_referencing;

#[self_referencing]
struct OwningCharIter {
    source_file: SourceFile,
    #[borrows(source_file)]
    #[covariant]
    chars: Chars<'this>,
}

impl OwningCharIter {
    fn from_source_file(source_file: &SourceFile) -> Self {
        OwningCharIterBuilder {
            source_file: source_file.clone(),
            chars_builder: |s| s.source_code().chars(),
        }
        .build()
    }

    #[inline]
    fn next(&mut self) -> Option<char> {
        self.with_chars_mut(|chars| chars.next())
    }
}

impl Debug for OwningCharIter {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Lexer {
    chars_left: i64,
    char_iter: OwningCharIter,
    source_file: SourceFile,
    current_offset: ByteOffset,
    start_offset: ByteOffset,
    lexeme_collector: Vec<char>,
    current_char: char,
    next_char: char,
    next_next_char: char,
}

static KEYWORDS: phf::Map<&'static str, TokenType> = phf_map! {
    "else" => TokenType::Else,
    "false" => TokenType::False,
    "for" => TokenType::For,
    "fun" => TokenType::Fun,
    "if" => TokenType::If,
    "return" => TokenType::Return,
    "true" => TokenType::True,
    "let" => TokenType::Let,
    "while" => TokenType::While,
    "struct" => TokenType::Struct,
};

impl Lexer {
    pub(crate) fn emit_token(&mut self, token_type: TokenType) -> Token {
        let token = Token {
            token_type,
            location: Location {
                source_file: self.source_file.clone(),
                start_byte: self.start_offset,
                end_byte: self.current_offset,
            },
            value: None,
        };
        self.start_offset = self.current_offset;
        self.lexeme_collector.clear();
        token
    }

    pub(crate) fn advance(&mut self) -> char {
        self.chars_left -= 1;
        self.current_char = self.next_char;
        self.current_offset += self.current_char.len_utf8() as i32;
        self.next_char = self.next_next_char;
        if self.chars_left > 0 {
            if let Some(char) = self.char_iter.next() {
                self.next_next_char = char;
            } else {
                self.next_next_char = '\0';
                self.chars_left = 1;
            }
        }
        self.lexeme_collector.push(self.current_char);
        self.current_char
    }

    pub(crate) fn matches(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.next_char != expected {
            return false;
        }
        self.advance();
        true
    }

    pub(crate) fn lex_string(&mut self) -> Token {
        while self.next_char != '"' && !self.is_at_end() {
            self.advance();
        }

        // closing "
        self.advance();
        self.emit_token(TokenType::String)
    }

    pub(crate) fn lex_number(&mut self) -> Token {
        while is_digit(self.next_char) {
            self.advance();
        }
        if self.next_char == '.' && is_digit(self.next_next_char) {
            // Consume the "."
            self.advance();
            while is_digit(self.next_char) {
                self.advance();
            }
        }

        self.emit_token(TokenType::Number)
    }

    pub(crate) fn lex_identifier_or_keyword(&mut self) -> Token {
        while is_alpha_numeric(self.next_char) {
            self.advance();
        }
        let mut token = self.emit_token(TokenType::Identifier);
        if let Some(token_type) = KEYWORDS.get(token.lexeme()) {
            // Keyword
            token.token_type = *token_type;
        }
        token
    }

    fn is_at_end(&self) -> bool {
        self.chars_left <= 0
    }

    fn ignore_chars(&mut self) {
        self.start_offset = self.current_offset;
        self.lexeme_collector.clear();
    }

    pub fn new(source_file: SourceFile) -> FelicoResult<Self> {
        let mut char_iter = OwningCharIter::from_source_file(&source_file);
        let mut next_char = '\0';
        let mut next_next_char = '\0';
        let mut chars_left = i64::MAX;
        if let Some(char) = char_iter.next() {
            next_char = char;
            if let Some(char) = char_iter.next() {
                next_next_char = char;
            } else {
                chars_left = 1;
            }
        } else {
            chars_left = 0;
        }
        Ok(Self {
            char_iter,
            source_file,
            current_offset: 0,
            start_offset: 0,
            lexeme_collector: Default::default(),
            current_char: '\0',
            next_char,
            next_next_char,
            chars_left,
        })
    }
}

impl Iterator for Lexer {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.chars_left <= 0 {
                return if self.chars_left < 0 {
                    None
                } else {
                    self.chars_left -= 1;
                    Some(self.emit_token(TokenType::EOF))
                };
            }
            let c = self.advance();
            let token_type = match c {
                '(' => TokenType::LeftParen,
                ')' => TokenType::RightParen,
                '{' => TokenType::LeftBrace,
                '}' => TokenType::RightBrace,
                ',' => TokenType::Comma,
                '.' => TokenType::Dot,
                '+' => TokenType::Plus,
                '-' => {
                    if self.matches('>') {
                        TokenType::Arrow
                    } else {
                        TokenType::Minus
                    }
                }
                ':' => TokenType::Colon,
                ';' => TokenType::Semicolon,
                '*' => TokenType::Star,
                '!' => {
                    if self.matches('=') {
                        TokenType::BangEqual
                    } else {
                        TokenType::Bang
                    }
                }
                '=' => {
                    if self.matches('=') {
                        TokenType::EqualEqual
                    } else {
                        TokenType::Equal
                    }
                }
                '<' => {
                    if self.matches('=') {
                        TokenType::LessEqual
                    } else {
                        TokenType::Less
                    }
                }
                '>' => {
                    if self.matches('=') {
                        TokenType::GreaterEqual
                    } else {
                        TokenType::Greater
                    }
                }
                '&' => {
                    if self.matches('&') {
                        TokenType::And
                    } else {
                        TokenType::UnexpectedCharacter
                    }
                }
                '|' => {
                    if self.matches('|') {
                        TokenType::Or
                    } else {
                        TokenType::UnexpectedCharacter
                    }
                }
                '/' => {
                    if self.matches('/') {
                        while self.next_char != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                        self.ignore_chars();
                        continue;
                    } else {
                        TokenType::Slash
                    }
                }
                '0'..='9' => return Some(self.lex_number()),
                'a'..='z' | 'A'..='Z' | '_' => return Some(self.lex_identifier_or_keyword()),
                ' ' | '\t' | '\r' => {
                    self.ignore_chars();
                    continue;
                }
                '\n' => {
                    self.ignore_chars();
                    continue;
                }
                '"' => {
                    return Some(self.lex_string());
                }
                _ => TokenType::UnexpectedCharacter,
            };
            return Some(self.emit_token(token_type));
        }
    }
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn is_alpha(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_alpha_numeric(c: char) -> bool {
    is_alpha(c) || is_digit(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{expect, Expect};
    /*

    TODO: test non-utf8
        #[test]
        fn lex_non_utf8() {
            let result = Lexer::new(SourceFileHandle::from_string("foo", || Box::new(Cursor::new(b"\xc3\x28")))));
            let err = result.err().expect("Should error");
            let expected = expect!["Could not read file 'foo': Could not read to string: stream did not contain valid UTF-8"];
            expected.assert_eq(&err.to_string());
        }
    */
    fn test_lexing(name: &str, input: &str, expected: Expect) {
        let s = Lexer::new(SourceFile::from_string(name, input)).unwrap();
        let result = s.collect::<Vec<_>>();
        let result_tokens = result
            .iter()
            .map(|token| {
                format!(
                    "{:<10} '{}' {}+{}",
                    token.token_type,
                    token.lexeme(),
                    token.location.start_byte,
                    token.location.end_byte - token.location.start_byte
                )
            })
            .fold(String::new(), |a, b| a + &b + "\n");
        expected.assert_eq(&result_tokens);
    }

    macro_rules! test_lex {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_lexing(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_lex!(
        empty: "" => expect![[r#"
            EOF        '' 0+0
        "#]];
        space : " " => expect![[r#"
            EOF        '' 1+0
        "#]];
        tab : "\t" => expect![[r#"
            EOF        '' 1+0
        "#]];
        newline : "\n" => expect![[r#"
            EOF        '' 1+0
        "#]];
        newline_windows : "\r\n" => expect![[r#"
            EOF        '' 2+0
        "#]];

        left_paren: "(" => expect![[r#"
            LeftParen  '(' 0+1
            EOF        '' 1+0
        "#]];
        newline2 : "\n(" => expect![[r#"
            LeftParen  '(' 1+1
            EOF        '' 2+0
        "#]];
        newline3 : "(\n)" => expect![[r#"
            LeftParen  '(' 0+1
            RightParen ')' 2+1
            EOF        '' 3+0
        "#]];

        bang: "!" => expect![[r#"
            Bang       '!' 0+1
            EOF        '' 1+0
        "#]];
        bang_equal: "!=" => expect![[r#"
            BangEqual  '!=' 0+2
            EOF        '' 2+0
        "#]];
        bang_equal2: "! =" => expect![[r#"
            Bang       '!' 0+1
            Equal      '=' 2+1
            EOF        '' 3+0
        "#]];

        equal: "=" => expect![[r#"
            Equal      '=' 0+1
            EOF        '' 1+0
        "#]];
        equal_equal: "==" => expect![[r#"
            EqualEqual '==' 0+2
            EOF        '' 2+0
        "#]];
        equal_equal2: "= =" => expect![[r#"
            Equal      '=' 0+1
            Equal      '=' 2+1
            EOF        '' 3+0
        "#]];

        slash: "/" => expect![[r#"
            Slash      '/' 0+1
            EOF        '' 1+0
        "#]];

        colon: ":" => expect![[r#"
            Colon      ':' 0+1
            EOF        '' 1+0
        "#]];
        slash_slash: "/ /" => expect![[r#"
            Slash      '/' 0+1
            Slash      '/' 2+1
            EOF        '' 3+0
        "#]];
        minus_greater: "- >" => expect![[r#"
            Minus      '-' 0+1
            Greater    '>' 2+1
            EOF        '' 3+0
        "#]];
        arrow: "->" => expect![[r#"
            Arrow      '->' 0+2
            EOF        '' 2+0
        "#]];
        comment: "//" => expect![[r#"
            EOF        '' 2+0
        "#]];
        comment2: "// bar" => expect![[r#"
            EOF        '' 6+0
        "#]];
        comment3: "// bar\n" => expect![[r#"
            EOF        '' 7+0
        "#]];


        // TODO: Test string lexemes
        empty_string: r#""""# => expect![[r#"
            String     '""' 0+2
            EOF        '' 2+0
        "#]];
        simple_string: r#""foobar""# => expect![[r#"
            String     '"foobar"' 0+8
            EOF        '' 8+0
        "#]];
        newline_string: r#""
                                ""# => expect![[r#"
                                    String     '"
                                                                    "' 0+35
                                    EOF        '' 35+0
                                "#]];
        string_and_bang: r#""x"!"# => expect![[r#"
            String     '"x"' 0+3
            Bang       '!' 3+1
            EOF        '' 4+0
        "#]];

        number_0: "0" => expect![[r#"
            Number     '0' 0+1
            EOF        '' 1+0
        "#]];
        number_1: "1" => expect![[r#"
            Number     '1' 0+1
            EOF        '' 1+0
        "#]];
        number_123: "123" => expect![[r#"
            Number     '123' 0+3
            EOF        '' 3+0
        "#]];

        number_98765434210: "98765434210" => expect![[r#"
            Number     '98765434210' 0+11
            EOF        '' 11+0
        "#]];

        numbers_98765434210: "9 8 7 6 5 4 3 4 2 1 0" => expect![[r#"
            Number     '9' 0+1
            Number     '8' 2+1
            Number     '7' 4+1
            Number     '6' 6+1
            Number     '5' 8+1
            Number     '4' 10+1
            Number     '3' 12+1
            Number     '4' 14+1
            Number     '2' 16+1
            Number     '1' 18+1
            Number     '0' 20+1
            EOF        '' 21+0
        "#]];

       decimal_numbers_0_0: "0.0" => expect![[r#"
           Number     '0.0' 0+3
           EOF        '' 3+0
       "#]];
       decimal_numbers_1_0: "1.0" => expect![[r#"
           Number     '1.0' 0+3
           EOF        '' 3+0
       "#]];
       decimal_numbers_9_9: "1.0" => expect![[r#"
           Number     '1.0' 0+3
           EOF        '' 3+0
       "#]];

       decimal_numbers_tau: "6.283185307179586" => expect![[r#"
           Number     '6.283185307179586' 0+17
           EOF        '' 17+0
       "#]];

       decimal_numbers_e: "2.71828" => expect![[r#"
           Number     '2.71828' 0+7
           EOF        '' 7+0
       "#]];

       numbers_1_dot_bang: "1.!" => expect![[r#"
           Number     '1' 0+1
           Dot        '.' 1+1
           Bang       '!' 2+1
           EOF        '' 3+0
       "#]];

        identifier_foo: "foo" => expect![[r#"
            Identifier 'foo' 0+3
            EOF        '' 3+0
        "#]];

        identifier_uppercase: "ZOO" => expect![[r#"
            Identifier 'ZOO' 0+3
            EOF        '' 3+0
        "#]];

        identifier_starting_with_underscore: "_foo_" => expect![[r#"
            Identifier '_foo_' 0+5
            EOF        '' 5+0
        "#]];

        identifier_almost_keyword: "anD" => expect![[r#"
            Identifier 'anD' 0+3
            EOF        '' 3+0
        "#]];

        identifier_almost_keyword2: "AND" => expect![[r#"
            Identifier 'AND' 0+3
            EOF        '' 3+0
        "#]];

        identifier_almost_keyword3: "and_" => expect![[r#"
            Identifier 'and_' 0+4
            EOF        '' 4+0
        "#]];

        keyword_and: "&&" => expect![[r#"
            And        '&&' 0+2
            EOF        '' 2+0
        "#]];


        keyword_else: "else" => expect![[r#"
            Else       'else' 0+4
            EOF        '' 4+0
        "#]];
        keyword_struct: "struct" => expect![[r#"
            Struct     'struct' 0+6
            EOF        '' 6+0
        "#]];

        keyword_false: "false" => expect![[r#"
            False      'false' 0+5
            EOF        '' 5+0
        "#]];

        keyword_for: "for" => expect![[r#"
            For        'for' 0+3
            EOF        '' 3+0
        "#]];

        keyword_fun: "fun" => expect![[r#"
            Fun        'fun' 0+3
            EOF        '' 3+0
        "#]];

        keyword_if: "if" => expect![[r#"
            If         'if' 0+2
            EOF        '' 2+0
        "#]];

        keyword_or: "||" => expect![[r#"
            Or         '||' 0+2
            EOF        '' 2+0
        "#]];

        keyword_print: "print" => expect![[r#"
            Identifier 'print' 0+5
            EOF        '' 5+0
        "#]];

        keyword_return: "return" => expect![[r#"
            Return     'return' 0+6
            EOF        '' 6+0
        "#]];

        keyword_true: "true" => expect![[r#"
            True       'true' 0+4
            EOF        '' 4+0
        "#]];

        keyword_let: "let" => expect![[r#"
            Let        'let' 0+3
            EOF        '' 3+0
        "#]];

        keyword_while: "while" => expect![[r#"
            While      'while' 0+5
            EOF        '' 5+0
        "#]];

        offset_simple: "\"x\"if" => expect![[r#"
            String     '"x"' 0+3
            If         'if' 3+2
            EOF        '' 5+0
        "#]];

        offset_two_bytes: "\"¤\"if" => expect![[r#"
            String     '"¤"' 0+4
            If         'if' 4+2
            EOF        '' 6+0
        "#]];
        offset_three_bytes: "\"⌚\"if" => expect![[r#"
            String     '"⌚"' 0+5
            If         'if' 5+2
            EOF        '' 7+0
        "#]];
        offset_four_bytes: "\"𝅘𝅥𝅮\"if" => expect![[r#"
            String     '"𝅘𝅥𝅮"' 0+6
            If         'if' 6+2
            EOF        '' 8+0
        "#]];
        offset_four_byte_emoji: "\"🦀\"if" => expect![[r#"
            String     '"🦀"' 0+6
            If         'if' 6+2
            EOF        '' 8+0
        "#]];
        offset_combined_char: "\"👨‍👩\"if" => expect![[r#"
            String     '"👨‍👩"' 0+13
            If         'if' 13+2
            EOF        '' 15+0
        "#]];

    );
}
