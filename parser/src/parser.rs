use felico_ast::ast_node::AstNode;
use felico_ast::compilation_unit::{CompilationUnit, CompilationUnitNode};
use felico_ast::expression::{Expression, ExpressionNode};
use felico_ast::fun_definition::{FunDefinition, FunDefinitionNode};
use felico_ast::identifier::{Identifier, IdentifierNode};
use felico_ast::statement::{ExpressionStatement, Statement, StatementNode};
use felico_base::error::FelicoError;
use felico_base::result::FelicoResult;
use felico_base::test_print::TestPrint;
use felico_base::value::Value;
use felico_base::{bail, err};
use felico_source::file_location::FileLocation;
use felico_source::source_error::SourceError;
use felico_source::source_file::SourceFile;
use felico_source::source_message::{SourceLabel, SourceMessage};
use felico_source::source_snippet::SourceSnippet;
use felico_source::source_span::SourceSpan;
use felico_token::{Lexeme, Token, TokenIterator, TokenKind};

pub struct Parser<'source> {
    source_file: &'source SourceFile,
    current_token: Token<'source>,
    last_position: usize,
    tokens: TokenIterator<'source>,
}

impl<'source> Parser<'source> {
    pub fn new(
        source_file: &'source SourceFile,
        mut tokens: TokenIterator<'source>,
    ) -> FelicoResult<Self> {
        let current_token = tokens
            .next()
            .ok_or_else(|| err!("No token in source file, expected at least EOF"))??;
        Ok(Self {
            source_file,
            tokens,
            current_token,
            last_position: 0,
        })
    }
}

impl<'source> Parser<'source> {
    pub fn parse(&mut self) -> FelicoResult<CompilationUnitNode<'source>> {
        self.parse_compilation_unit()
    }

    pub fn parse_script(&mut self) -> FelicoResult<CompilationUnitNode<'source>> {
        let start_position = self.current_position();
        let name = self.create_node(start_position, Identifier::new("script".to_string()))?;
        let statements = self.parse_statements(TokenKind::EOF)?;
        let script_function =
            self.create_node(start_position, FunDefinition::new(name, statements))?;
        self.create_node(start_position, CompilationUnit::new(vec![script_function]))
    }

    fn advance(&mut self) -> FelicoResult<Token<'source>> {
        self.last_position = self.current_token.location.end;
        let mut token = self
            .tokens
            .next()
            .ok_or_else(|| err!("No more token in source file, expected at least EOF"))??;
        std::mem::swap(&mut self.current_token, &mut token);
        Ok(token)
    }

    fn consume(&mut self, token_kind: TokenKind) -> FelicoResult<Token<'source>> {
        if self.current_token.kind != token_kind {
            return self.create_token_error(
                format!(
                    "Unexpected token: {}, expected {}",
                    self.current_token, token_kind
                ),
                format!("expected {token_kind} here"),
            );
        }
        let token = self.advance()?;
        Ok(token)
    }

    fn create_token_error<T>(
        &mut self,
        error_message: String,
        token_label: String,
    ) -> FelicoResult<T> {
        Err(self.create_token_error_internal(error_message, token_label))
    }

    fn create_token_error_internal(
        &mut self,
        error_message: String,
        token_label: String,
    ) -> FelicoError {
        let source_snippet = SourceSnippet::new(
            self.source_file.path().to_string(),
            self.source_file.content().to_string(),
            1,
            0,
        );
        let mut source_message = SourceMessage::error(error_message, source_snippet);
        source_message.add_label(SourceLabel::new(
            SourceSpan::new(
                self.current_token.location.start,
                self.current_token.location.end,
            ),
            token_label,
        ));
        SourceError::new(source_message).into()
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
                other => bail!("Unexpected token: {other}"),
            }
            self.advance()?;
        }
        self.create_node(start_position, CompilationUnit::new(fun_definitions))
    }

    fn parse_function(&mut self) -> FelicoResult<FunDefinitionNode<'source>> {
        let start_position = self.current_position();
        self.consume(TokenKind::Fun)?;
        let name = self.parse_identifier()?;
        self.consume(TokenKind::ParenOpen)?;
        self.consume(TokenKind::ParenClose)?;
        self.consume(TokenKind::BraceOpen)?;
        let statements = self.parse_statements(TokenKind::BraceClose)?;
        self.consume(TokenKind::BraceClose)?;
        self.create_node(start_position, FunDefinition::new(name, statements))
    }

    fn parse_statements(
        &mut self,
        end_token_kind: TokenKind,
    ) -> Result<Vec<StatementNode<'source>>, FelicoError> {
        let mut statements = Vec::new();
        while self.current_token.kind != end_token_kind {
            let statement = self.parse_statement()?;
            statements.push(statement);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> FelicoResult<StatementNode<'source>> {
        let result = self.parse_expression_statement()?;
        self.consume(TokenKind::Semicolon)?;
        Ok(result)
    }

    fn parse_expression_statement(&mut self) -> FelicoResult<StatementNode<'source>> {
        let start_position = self.current_position();
        let expression = self.parse_expression()?;
        self.create_node(
            start_position,
            Statement::Expression(ExpressionStatement { expression }),
        )
    }

    fn parse_expression(&mut self) -> FelicoResult<ExpressionNode<'source>> {
        let expression = self.parse_call()?;
        Ok(expression)
    }

    fn parse_call(&mut self) -> FelicoResult<ExpressionNode<'source>> {
        let start_position = self.current_position();
        let expr = self.parse_primary_expression()?;
        if !self.is_at(TokenKind::ParenOpen) {
            return Ok(expr);
        }
        self.consume(TokenKind::ParenOpen)?;
        let argument = self.parse_expression()?;
        self.consume(TokenKind::ParenClose)?;
        self.create_node(start_position, Expression::call(expr, vec![argument]))
    }

    fn parse_primary_expression(&mut self) -> FelicoResult<ExpressionNode<'source>> {
        let start_position = self.current_position();
        let result = match self.current_token.kind {
            TokenKind::Identifier => {
                let name = self.parse_identifier()?;
                self.create_node(start_position, Expression::var_use(name))
            }
            TokenKind::String => {
                let token = self.consume(TokenKind::String)?;
                self.create_node(
                    start_position,
                    Expression::literal(extract_string_from_lexeme(token.lexeme)?),
                )
            }
            _other => self.create_token_error(
                format!("Unexpected token: {}", self.current_token),
                "expected primary expression here".to_string(),
            ),
        }?;
        Ok(result)
    }

    fn create_node<T: TestPrint>(
        &mut self,
        start_position: usize,
        node: T,
    ) -> FelicoResult<AstNode<'source, T>> {
        Ok(AstNode::new(
            FileLocation::new(self.source_file, start_position, self.last_position),
            node,
        ))
    }

    fn is_at(&mut self, token_kind: TokenKind) -> bool {
        self.current_token.kind == token_kind
    }

    fn current_position(&mut self) -> usize {
        self.current_token.location.start
    }

    fn parse_identifier(&mut self) -> FelicoResult<IdentifierNode<'source>> {
        let start_position = self.current_position();
        let name = self.consume(TokenKind::Identifier)?;
        self.create_node(start_position, Identifier::new(name.lexeme.to_string()))
    }
}

fn extract_string_from_lexeme(lexeme: Lexeme) -> FelicoResult<Value> {
    assert!(lexeme.starts_with('"') && lexeme.ends_with('"'));
    let string_content = &lexeme[1..lexeme.len() - 1];
    let string = if !string_content.contains("\\") {
        string_content.to_string()
    } else {
        // unescape backslash escape codes
        let mut unescaped = String::new();
        let mut chars = string_content.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.peek() {
                    Some('n') => {
                        unescaped.push('\n');
                        chars.next();
                    }
                    Some('t') => {
                        unescaped.push('\t');
                        chars.next();
                    }
                    Some('"') => {
                        unescaped.push('\"');
                        chars.next();
                    }
                    Some('\\') => {
                        unescaped.push('\\');
                        chars.next();
                    }
                    Some(other) => {
                        bail!("Invalid escape sequence: \\{other}")
                    }
                    None => bail!("Incomplete escape sequence"),
                }
            } else {
                unescaped.push(c);
            }
        }
        unescaped
    };
    Ok(Value::String(string))
}

#[cfg(test)]
mod tests {
    use crate::parser::{Parser, extract_string_from_lexeme};
    use expect_test::{Expect, expect};
    use felico_base::bail;
    use felico_base::result::FelicoResult;
    use felico_base::test_print::TestPrint;
    use felico_lexer::lexer::Lexer;
    use felico_source::source_file::SourceFile;

    fn test_parse_string_from_lexeme(lexeme: &str, expected: Expect) -> FelicoResult<()> {
        let value = extract_string_from_lexeme(lexeme)?;
        expected.assert_eq(&value.to_string());
        Ok(())
    }

    macro_rules! test_parse_string_from_lexeme {
        ($name:ident, $source:literal, $expected:expr) => {
            #[test]
            fn $name() -> FelicoResult<()> {
                test_parse_string_from_lexeme($source, $expected)
            }
        };
    }

    test_parse_string_from_lexeme!(string_empty, r#""""#, expect![[r#""""#]]);
    test_parse_string_from_lexeme!(string_simple, r#""foo""#, expect![[r#""foo""#]]);
    test_parse_string_from_lexeme!(
        string_escapes,
        "\"newline\\ntab\\tbackslash\\\"\"",
        expect![[r#"
        "newline
        tab	backslash"""#]]
    );

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
            🌲   0+19  fun ❮do_nothing❯
        "#]]
    );

    test_parse!(
        fun_call,
        "fun foo() {print(\"hello\"   );}",
        expect![[r#"
            🌲   0+30  Compilation Unit
            🌲   0+30  fun ❮foo❯
            🌲  11+17   stmt  call  var use ❮print❯
            🌲  17+7       literal "hello"
        "#]]
    );

    fn test_parse_script(source: &str, expected: Expect) -> FelicoResult<()> {
        let source_file = SourceFile::in_memory("script.felico", source);
        let lexer = Lexer::new(&source_file);
        let mut parser = Parser::new(&source_file, Box::new(lexer))?;
        let result = parser.parse_script()?;
        let mut test_string = String::new();
        result.test_print(&mut test_string, 0)?;
        expected.assert_eq(&test_string);
        Ok(())
    }

    macro_rules! test_parse_script {
        ($name:ident, $source:literal, $expected:expr) => {
            #[test]
            fn $name() -> FelicoResult<()> {
                test_parse_script($source, $expected)
            }
        };
    }

    test_parse_script!(
        script_empty,
        "",
        expect![[r#"
            🌲   0+0   Compilation Unit
            🌲   0+0   fun ❮script❯
        "#]]
    );

    test_parse_script!(
        script_print,
        "print(\"hello\");",
        expect![[r#"
            🌲   0+15  Compilation Unit
            🌲   0+15  fun ❮script❯
            🌲   0+14   stmt  call  var use ❮print❯
            🌲   6+7       literal "hello"
        "#]]
    );

    test_parse_script!(
        script_print_twice,
        r#"
            print("hello");
            print("world");
        "#,
        expect![[r#"
            🌲  13+43  Compilation Unit
            🌲  13+43  fun ❮script❯
            🌲  13+14   stmt  call  var use ❮print❯
            🌲  19+7       literal "hello"
            🌲  41+14   stmt  call  var use ❮print❯
            🌲  47+7       literal "world"
        "#]]
    );

    fn test_parse_error(source: &str, expected: Expect) -> FelicoResult<()> {
        let source_file = SourceFile::in_memory("test.felico", source);
        let lexer = Lexer::new(&source_file);
        let mut parser = Parser::new(&source_file, Box::new(lexer))?;
        let Err(error) = parser.parse() else {
            bail!("expected error")
        };
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
            Error: error: Unexpected token: “(” (Open Parenthesis), expected Identifier
              ╭▸ test.felico:1:5
              │
            1 │ fun () {}
              ╰╴    ━ expected Identifier here
        "#]]
    );

    fn test_parse_script_error(source: &str, expected: Expect) -> FelicoResult<()> {
        let source_file = SourceFile::in_memory("script.felico", source);
        let lexer = Lexer::new(&source_file);
        let mut parser = Parser::new(&source_file, Box::new(lexer))?;
        let Err(error) = parser.parse_script() else {
            bail!("expected error")
        };
        expected.assert_eq(&error.to_test_string());
        Ok(())
    }

    macro_rules! test_parse_script_error {
        ($name:ident, $source:literal, $expected:expr) => {
            #[test]
            fn $name() -> FelicoResult<()> {
                test_parse_script_error($source, $expected)
            }
        };
    }

    test_parse_script_error!(
        error_no_expression,
        "}",
        expect![[r#"
            Error: error: Unexpected token: “}” (Close Brace)
              ╭▸ script.felico:1:1
              │
            1 │ }
              ╰╴━ expected primary expression here
        "#]]
    );
}
