use phf::phf_map;
use std::fmt::{Debug, Formatter};
use std::str::Chars;

use crate::frontend::lexer::token::{Token, TokenType};
use crate::infra::result::FelicoResult;
use crate::infra::location::{Location, ByteOffset};
use crate::infra::source_file::SourceFileHandle;
use ouroboros::self_referencing;

#[self_referencing]
struct OwningCharIter {
    source_file: SourceFileHandle,
    #[borrows(source_file)]
    #[covariant]
    chars: Chars<'this>,
}

impl OwningCharIter {
    fn from_source_file(source_file: &SourceFileHandle) -> Self {
        OwningCharIterBuilder {
            source_file: source_file.clone(),
            chars_builder: |s| s.source_code().chars(),
        }.build()
    }

    #[inline]
    fn next(&mut self) -> Option<char>{
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
    source_file: SourceFileHandle,
    current_offset: ByteOffset,
    start_offset: ByteOffset,
    line_number: ByteOffset,
    start_line_number: ByteOffset,
    start_column_number: ByteOffset,
    column_number: ByteOffset,
    lexeme_collector: Vec<char>,
    current_char: char,
    next_char: char,
    next_next_char: char,
}

static KEYWORDS: phf::Map<&'static str, TokenType> = phf_map! {
    "and" => TokenType::And,
    "class" => TokenType::Class,
    "else" => TokenType::Else,
    "false" => TokenType::False,
    "for" => TokenType::For,
    "fun" => TokenType::Fun,
    "if" => TokenType::If,
    "nil" => TokenType::Nil,
    "or" => TokenType::Or,
    "print" => TokenType::Print,
    "return" => TokenType::Return,
    "super" => TokenType::Super,
    "this" => TokenType::This,
    "true" => TokenType::True,
    "var" => TokenType::Var,
    "while" => TokenType::While,
};

impl Lexer {
    pub(crate) fn emit_token(&mut self, token_type: TokenType) -> Token {
        let token = Token {
            token_type,
            location: Location {
                source_file: self.source_file.clone(),
                start_byte: self.start_offset,
                end_byte: self.current_offset,
                line: self.start_line_number,
                column: self.start_column_number + 1,
            },
        };
        self.start_offset = self.current_offset;
        self.start_column_number = self.column_number;
        self.start_line_number = self.line_number;
        self.lexeme_collector.clear();
        token
    }

    pub(crate) fn advance(&mut self) -> char {
        self.chars_left -= 1;
        self.column_number += 1;
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
            if self.current_char == '\n' {
                self.column_number = 0;
                self.line_number += 1;
            }
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
        self.start_column_number = self.column_number;
        self.start_line_number = self.line_number;
        self.lexeme_collector.clear();
    }

    pub fn new(source_file: SourceFileHandle) -> FelicoResult<Self> {
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
            line_number: 1,
            start_line_number: 1,
            column_number: 0,
            start_column_number: 0,
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
                '-' => TokenType::Minus,
                ';' => TokenType::Semicolon,
                '*' => TokenType::Star,
                '!' => if self.matches('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                }
                '=' => if self.matches('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                }
                '<' => if self.matches('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                }
                '>' => if self.matches('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                }
                '/' => if self.matches('/') {
                    while self.next_char != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                    self.ignore_chars();
                    continue;
                } else {
                    TokenType::Slash
                }
                '0'..='9' => {
                    return Some(self.lex_number())
                }
                'a'..='z'|'A'..='Z'|'_' => {
                    return Some(self.lex_identifier_or_keyword())
                }
                ' ' | '\t' | '\r' => {
                    self.ignore_chars();
                    continue;
                }
                '\n' => {
                    self.column_number = 0;
                    self.line_number += 1;
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
    c >= '0' && c <= '9'
}

fn is_alpha(c: char) -> bool {
    (c >= 'a' && c <= 'z') ||
        (c >= 'A' && c <= 'Z') ||
        c == '_'
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
        let s = Lexer::new(SourceFileHandle::from_string(name, input)).unwrap();
        let result = s.collect::<Vec<_>>();
        let result_tokens = result.iter().map(|token| format!("{:<10} '{}' {}:{} ({}+{})", token.token_type, token.lexeme(), token.location.line, token.location.column, token.location.start_byte, token.location.end_byte - token.location.start_byte)).fold(String::new(), |a, b| a + &b + "\n");
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
            EOF        '' 1:1 (0+0)
        "#]];
        space : " " => expect![[r#"
            EOF        '' 1:2 (1+0)
        "#]];
        tab : "\t" => expect![[r#"
            EOF        '' 1:2 (1+0)
        "#]];
        newline : "\n" => expect![[r#"
            EOF        '' 2:1 (1+0)
        "#]];
        newline_windows : "\r\n" => expect![[r#"
            EOF        '' 2:1 (2+0)
        "#]];

        left_paren: "(" => expect![[r#"
            LeftParen  '(' 1:1 (0+1)
            EOF        '' 1:2 (1+0)
        "#]];
        newline2 : "\n(" => expect![[r#"
            LeftParen  '(' 2:1 (1+1)
            EOF        '' 2:2 (2+0)
        "#]];
        newline3 : "(\n)" => expect![[r#"
            LeftParen  '(' 1:1 (0+1)
            RightParen ')' 2:1 (2+1)
            EOF        '' 2:2 (3+0)
        "#]];

        bang: "!" => expect![[r#"
            Bang       '!' 1:1 (0+1)
            EOF        '' 1:2 (1+0)
        "#]];
        bang_equal: "!=" => expect![[r#"
            BangEqual  '!=' 1:1 (0+2)
            EOF        '' 1:3 (2+0)
        "#]];
        bang_equal2: "! =" => expect![[r#"
            Bang       '!' 1:1 (0+1)
            Equal      '=' 1:3 (2+1)
            EOF        '' 1:4 (3+0)
        "#]];

        equal: "=" => expect![[r#"
            Equal      '=' 1:1 (0+1)
            EOF        '' 1:2 (1+0)
        "#]];
        equal_equal: "==" => expect![[r#"
            EqualEqual '==' 1:1 (0+2)
            EOF        '' 1:3 (2+0)
        "#]];
        equal_equal2: "= =" => expect![[r#"
            Equal      '=' 1:1 (0+1)
            Equal      '=' 1:3 (2+1)
            EOF        '' 1:4 (3+0)
        "#]];

        slash: "/" => expect![[r#"
            Slash      '/' 1:1 (0+1)
            EOF        '' 1:2 (1+0)
        "#]];
        slash_slash: "/ /" => expect![[r#"
            Slash      '/' 1:1 (0+1)
            Slash      '/' 1:3 (2+1)
            EOF        '' 1:4 (3+0)
        "#]];
        comment: "//" => expect![[r#"
            EOF        '' 1:3 (2+0)
        "#]];
        comment2: "// bar" => expect![[r#"
            EOF        '' 1:7 (6+0)
        "#]];
        comment3: "// bar\n" => expect![[r#"
            EOF        '' 2:1 (7+0)
        "#]];


        // TODO: Test string lexemes
        empty_string: r#""""# => expect![[r#"
            String     '""' 1:1 (0+2)
            EOF        '' 1:3 (2+0)
        "#]];
        simple_string: r#""foobar""# => expect![[r#"
            String     '"foobar"' 1:1 (0+8)
            EOF        '' 1:9 (8+0)
        "#]];
        newline_string: r#""
                                ""# => expect![[r#"
                                    String     '"
                                                                    "' 1:1 (0+35)
                                    EOF        '' 2:34 (35+0)
                                "#]];
        string_and_bang: r#""x"!"# => expect![[r#"
            String     '"x"' 1:1 (0+3)
            Bang       '!' 1:4 (3+1)
            EOF        '' 1:5 (4+0)
        "#]];

        number_0: "0" => expect![[r#"
            Number     '0' 1:1 (0+1)
            EOF        '' 1:2 (1+0)
        "#]];
        number_1: "1" => expect![[r#"
            Number     '1' 1:1 (0+1)
            EOF        '' 1:2 (1+0)
        "#]];
        number_123: "123" => expect![[r#"
            Number     '123' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        number_98765434210: "98765434210" => expect![[r#"
            Number     '98765434210' 1:1 (0+11)
            EOF        '' 1:12 (11+0)
        "#]];

        numbers_98765434210: "9 8 7 6 5 4 3 4 2 1 0" => expect![[r#"
            Number     '9' 1:1 (0+1)
            Number     '8' 1:3 (2+1)
            Number     '7' 1:5 (4+1)
            Number     '6' 1:7 (6+1)
            Number     '5' 1:9 (8+1)
            Number     '4' 1:11 (10+1)
            Number     '3' 1:13 (12+1)
            Number     '4' 1:15 (14+1)
            Number     '2' 1:17 (16+1)
            Number     '1' 1:19 (18+1)
            Number     '0' 1:21 (20+1)
            EOF        '' 1:22 (21+0)
        "#]];

       decimal_numbers_0_0: "0.0" => expect![[r#"
           Number     '0.0' 1:1 (0+3)
           EOF        '' 1:4 (3+0)
       "#]];
       decimal_numbers_1_0: "1.0" => expect![[r#"
           Number     '1.0' 1:1 (0+3)
           EOF        '' 1:4 (3+0)
       "#]];
       decimal_numbers_9_9: "1.0" => expect![[r#"
           Number     '1.0' 1:1 (0+3)
           EOF        '' 1:4 (3+0)
       "#]];

       decimal_numbers_tau: "6.283185307179586" => expect![[r#"
           Number     '6.283185307179586' 1:1 (0+17)
           EOF        '' 1:18 (17+0)
       "#]];

       decimal_numbers_e: "2.71828" => expect![[r#"
           Number     '2.71828' 1:1 (0+7)
           EOF        '' 1:8 (7+0)
       "#]];

       numbers_1_dot_bang: "1.!" => expect![[r#"
           Number     '1' 1:1 (0+1)
           Dot        '.' 1:2 (1+1)
           Bang       '!' 1:3 (2+1)
           EOF        '' 1:4 (3+0)
       "#]];

        identifier_foo: "foo" => expect![[r#"
            Identifier 'foo' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        identifier_uppercase: "ZOO" => expect![[r#"
            Identifier 'ZOO' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        identifier_starting_with_underscore: "_foo_" => expect![[r#"
            Identifier '_foo_' 1:1 (0+5)
            EOF        '' 1:6 (5+0)
        "#]];

        identifier_almost_keyword: "anD" => expect![[r#"
            Identifier 'anD' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        identifier_almost_keyword2: "AND" => expect![[r#"
            Identifier 'AND' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        identifier_almost_keyword3: "and_" => expect![[r#"
            Identifier 'and_' 1:1 (0+4)
            EOF        '' 1:5 (4+0)
        "#]];

        keyword_and: "and" => expect![[r#"
            And        'and' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        keyword_class: "class" => expect![[r#"
            Class      'class' 1:1 (0+5)
            EOF        '' 1:6 (5+0)
        "#]];

        keyword_else: "class" => expect![[r#"
            Class      'class' 1:1 (0+5)
            EOF        '' 1:6 (5+0)
        "#]];

        keyword_false: "false" => expect![[r#"
            False      'false' 1:1 (0+5)
            EOF        '' 1:6 (5+0)
        "#]];

        keyword_for: "for" => expect![[r#"
            For        'for' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        keyword_fun: "fun" => expect![[r#"
            Fun        'fun' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        keyword_íf: "if" => expect![[r#"
            If         'if' 1:1 (0+2)
            EOF        '' 1:3 (2+0)
        "#]];

        keyword_nil: "nil" => expect![[r#"
            Nil        'nil' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        keyword_or: "or" => expect![[r#"
            Or         'or' 1:1 (0+2)
            EOF        '' 1:3 (2+0)
        "#]];

        keyword_print: "print" => expect![[r#"
            Print      'print' 1:1 (0+5)
            EOF        '' 1:6 (5+0)
        "#]];

        keyword_return: "return" => expect![[r#"
            Return     'return' 1:1 (0+6)
            EOF        '' 1:7 (6+0)
        "#]];

        keyword_super: "super" => expect![[r#"
            Super      'super' 1:1 (0+5)
            EOF        '' 1:6 (5+0)
        "#]];

        keyword_this: "this" => expect![[r#"
            This       'this' 1:1 (0+4)
            EOF        '' 1:5 (4+0)
        "#]];

        keyword_true: "true" => expect![[r#"
            True       'true' 1:1 (0+4)
            EOF        '' 1:5 (4+0)
        "#]];

        keyword_var: "var" => expect![[r#"
            Var        'var' 1:1 (0+3)
            EOF        '' 1:4 (3+0)
        "#]];

        keyword_while: "while" => expect![[r#"
            While      'while' 1:1 (0+5)
            EOF        '' 1:6 (5+0)
        "#]];

        offset_simple: "\"x\"if" => expect![[r#"
            String     '"x"' 1:1 (0+3)
            If         'if' 1:4 (3+2)
            EOF        '' 1:6 (5+0)
        "#]];

        offset_two_bytes: "\"¤\"if" => expect![[r#"
            String     '"¤"' 1:1 (0+4)
            If         'if' 1:4 (4+2)
            EOF        '' 1:6 (6+0)
        "#]];
        offset_three_bytes: "\"⌚\"if" => expect![[r#"
            String     '"⌚"' 1:1 (0+5)
            If         'if' 1:4 (5+2)
            EOF        '' 1:6 (7+0)
        "#]];
        offset_four_bytes: "\"𝅘𝅥𝅮\"if" => expect![[r#"
            String     '"𝅘𝅥𝅮"' 1:1 (0+6)
            If         'if' 1:4 (6+2)
            EOF        '' 1:6 (8+0)
        "#]];
        offset_four_byte_emoji: "\"🦀\"if" => expect![[r#"
            String     '"🦀"' 1:1 (0+6)
            If         'if' 1:4 (6+2)
            EOF        '' 1:6 (8+0)
        "#]];
        offset_combined_char: "\"👨‍👩\"if" => expect![[r#"
            String     '"👨‍👩"' 1:1 (0+13)
            If         'if' 1:6 (13+2)
            EOF        '' 1:8 (15+0)
        "#]];

    );
}