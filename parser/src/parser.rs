use felico_ast::ast_node::AstNode;
use felico_ast::compilation_unit::{CompilationUnit, CompilationUnitNode};
use felico_ast::fun_definition::{FunDefinition, FunDefinitionNode};
use felico_ast::identifier::{Identifier, IdentifierNode};
use felico_base::error::FelicoError;
use felico_base::result::FelicoResult;
use felico_source::file_location::FileLocation;
use felico_source::source_error::SourceError;
use felico_source::source_file::SourceFile;
use felico_source::source_message::{SourceLabel, SourceMessage};
use felico_source::source_snippet::SourceSnippet;
use felico_source::source_span::SourceSpan;
use felico_token::{Token, TokenIterator, TokenKind};

pub struct Parser<'source> {
    source_file: &'source SourceFile,
    current_token: Token<'source>,
    tokens: TokenIterator<'source>,
}

impl<'source> Parser<'source> {
    pub fn new(
        source_file: &'source SourceFile,
        mut tokens: TokenIterator<'source>,
    ) -> FelicoResult<Self> {
        let current_token = tokens.next().ok_or_else(|| {
            FelicoError::message("No token in source file, expected at least EOF")
        })??;
        Ok(Self {
            source_file,
            tokens,
            current_token,
        })
    }
}

impl<'source> Parser<'source> {
    pub fn parse(&mut self) -> FelicoResult<CompilationUnitNode<'source>> {
        self.parse_compilation_unit()
    }

    fn advance(&mut self) -> FelicoResult<Token<'source>> {
        let mut token = self.tokens.next().ok_or_else(|| {
            FelicoError::message("No more token in source file, expected at least EOF")
        })??;
        std::mem::swap(&mut self.current_token, &mut token);
        Ok(token)
    }

    fn consume(&mut self, token_kind: TokenKind) -> FelicoResult<Token<'source>> {
        if self.current_token.kind != token_kind {
            let source_snippet = SourceSnippet::new(
                self.source_file.path().to_string(),
                self.source_file.content().to_string(),
                1,
                0,
            );
            let mut source_message = SourceMessage::error(
                format!(
                    "Unexpected token: “{}” ({}) , expected {}",
                    self.current_token.lexeme, self.current_token.kind, token_kind
                ),
                source_snippet,
            );
            source_message.add_label(SourceLabel::new(
                SourceSpan::new(
                    self.current_token.location.start,
                    self.current_token.location.end,
                ),
                format!("expected {token_kind} here",),
            ));
            return Err(SourceError::new(source_message).into());
        }
        let token = self.advance()?;
        Ok(token)
    }

    fn parse_compilation_unit(&mut self) -> FelicoResult<CompilationUnitNode<'source>> {
        let start_position = self.current_position();
        let mut fun_definitions = Vec::new();
        loop {
            match self.current_token.kind {
                TokenKind::EOF => break,
                TokenKind::Fun => {
                    fun_definitions.push(self.parse_function()?);
                }
                other => return Err(FelicoError::message(format!("Unexpected token: {other}"))),
            }
            self.advance()?;
        }
        Ok(AstNode::new(
            FileLocation::new(self.source_file, start_position, self.current_position()),
            CompilationUnit::new(fun_definitions),
        ))
    }

    fn parse_function(&mut self) -> FelicoResult<FunDefinitionNode<'source>> {
        let start_position = self.current_position();
        self.consume(TokenKind::Fun)?;
        let name = self.parse_identifier()?;
        self.consume(TokenKind::ParenOpen)?;
        self.consume(TokenKind::ParenClose)?;
        self.consume(TokenKind::BraceOpen)?;
        self.consume(TokenKind::BraceClose)?;
        Ok(AstNode::new(
            FileLocation::new(self.source_file, start_position, self.current_position()),
            FunDefinition::new(name),
        ))
    }

    fn current_position(&mut self) -> usize {
        self.current_token.location.start
    }

    fn parse_identifier(&mut self) -> FelicoResult<IdentifierNode<'source>> {
        let name = self.consume(TokenKind::Identifier)?;
        let file_location = FileLocation::new(self.source_file, 0, 0);
        Ok(AstNode::new(
            file_location,
            Identifier::new(name.lexeme.to_string()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use expect_test::{Expect, expect};
    use felico_ast::test_print::TestPrint;
    use felico_base::error::FelicoError;
    use felico_base::result::FelicoResult;
    use felico_lexer::lexer::Lexer;
    use felico_source::source_file::SourceFile;

    fn test_parse(source: &str, expected: Expect) -> FelicoResult<()> {
        let source_file = SourceFile::in_memory("test.felico", source);
        let lexer = Lexer::new(&source_file);
        let mut parser = Parser::new(&source_file, Box::new(lexer))?;
        let result = parser.parse()?;
        let mut test_string = String::new();
        result.test_print(&mut test_string, 0)?;
        expected.assert_eq(&test_string);
        Ok(())
    }

    macro_rules! test_parse {
        ($name:ident, $source:literal, $expected:expr) => {
            #[test]
            fn $name() -> FelicoResult<()> {
                test_parse($source, $expected)
            }
        };
    }

    test_parse!(
        empty,
        "",
        expect![[r#"
        🌲   0+0   Compilation Unit
    "#]]
    );

    test_parse!(
        fun_empty,
        "fun do_nothing() {}",
        expect![[r#"
        🌲   0+19  Compilation Unit
        🌲   0+19  fun do_nothing
    "#]]
    );

    fn test_parse_error(source: &str, expected: Expect) -> FelicoResult<()> {
        let source_file = SourceFile::in_memory("test.felico", source);
        let lexer = Lexer::new(&source_file);
        let mut parser = Parser::new(&source_file, Box::new(lexer))?;
        let Err(error) = parser.parse() else {
            return Err(FelicoError::message("expected error"));
        };
        dbg!(&error);
        expected.assert_eq(&error.to_test_string());
        Ok(())
    }

    macro_rules! test_parse_error {
        ($name:ident, $source:literal, $expected:expr) => {
            #[test]
            fn $name() -> FelicoResult<()> {
                test_parse_error($source, $expected)
            }
        };
    }

    test_parse_error!(
        error_fun_no_name,
        "fun () {}",
        expect![[r#"
        Error: error: Unexpected token: “(” (ParenOpen) , expected Identifier
          ╭▸ test.felico:1:5
          │
        1 │ fun () {}
          ╰╴    ━ expected Identifier here
    "#]]
    );
}
