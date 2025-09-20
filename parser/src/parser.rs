use felico_ast::ast_node::AstNode;
use felico_ast::compilation_unit::{CompilationUnit, CompilationUnitAst};
use felico_base::result::FelicoResult;
use felico_source::file_location::FileLocation;
use felico_source::source_file::SourceFile;
use felico_token::TokenIterator;

pub struct Parser<'source> {
    source_file: &'source SourceFile,
    tokens: TokenIterator<'source>,
}

impl<'source> Parser<'source> {
    pub fn new(source_file: &'source SourceFile, tokens: TokenIterator<'source>) -> Self {
        Self {
            source_file,
            tokens,
        }
    }
}

impl<'source> Parser<'source> {
    pub fn parse(&mut self) -> FelicoResult<CompilationUnitAst<'source>> {
        self.tokens.next();
        Ok(AstNode::new(
            FileLocation::new(self.source_file, 0, 0),
            CompilationUnit::new(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use expect_test::expect;
    use felico_ast::test_print::TestPrint;
    use felico_lexer::lexer::Lexer;
    use felico_source::source_file::SourceFile;

    #[test]
    fn test_parse_empty() {
        let source_file = SourceFile::in_memory("test.felico", "");
        let lexer = Lexer::new(&source_file);
        let mut parser = Parser::new(&source_file, Box::new(lexer));
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
}
