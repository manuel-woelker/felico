use felico_ast::ast_node::AstNode;
use felico_ast::compilation_unit::{CompilationUnit, CompilationUnitNode};
use felico_ast::fun_definition::{FunDefinition, FunDefinitionNode};
use felico_ast::identifier::{Identifier, IdentifierNode};
use felico_base::error::FelicoError;
use felico_base::result::FelicoResult;
use felico_source::file_location::FileLocation;
use felico_source::source_file::SourceFile;
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
            return Err(FelicoError::message(format!(
                "Unexpected token: {}, expected {}",
                self.current_token.kind, token_kind
            )));
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
    use expect_test::expect;
    use felico_ast::test_print::TestPrint;
    use felico_base::result::FelicoResult;
    use felico_lexer::lexer::Lexer;
    use felico_source::source_file::SourceFile;

    #[test]
    fn test_parse_empty() {
        let source_file = SourceFile::in_memory("test.felico", "");
        let lexer = Lexer::new(&source_file);
        let mut parser = Parser::new(&source_file, Box::new(lexer)).unwrap();
        let result = parser.parse().unwrap();
        assert_eq!(result.location.start, 0);
        assert_eq!(result.location.end, 0);
        let mut test_string = String::new();
        result.test_print(&mut test_string, 0).unwrap();
        expect!([r#"
            🌲   0+0   Compilation Unit
        "#])
        .assert_eq(&test_string);
    }

    #[test]
    fn test_parse_empty_fun() -> FelicoResult<()> {
        let source_file = SourceFile::in_memory("test.felico", "fun do_nothing() {}");
        let lexer = Lexer::new(&source_file);
        let mut parser = Parser::new(&source_file, Box::new(lexer))?;
        let result = parser.parse()?;
        let mut test_string = String::new();
        result.test_print(&mut test_string, 0)?;
        expect!([r#"
            🌲   0+19  Compilation Unit
            🌲   0+19  fun do_nothing
        "#])
        .assert_eq(&test_string);
        Ok(())
    }
}
