use crate::frontend::ast::expr::{
    AssignExpr, BinaryExpr, CallExpr, Expr, GetExpr, LiteralExpr, SetExpr, UnaryExpr, VarUse,
};
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::program::Program;
use crate::frontend::ast::stmt::{
    BlockStmt, ExprStmt, FunParameter, FunStmt, IfStmt, LetStmt, ReturnStmt, Stmt, StructStmt,
    StructStmtField, WhileStmt,
};
use crate::frontend::ast::AstData;
use crate::frontend::lex::lexer::Lexer;
use crate::frontend::lex::token::{Token, TokenType};
use crate::infra::diagnostic::InterpreterDiagnostic;
use crate::infra::location::Location;
use crate::infra::result::{bail, failed, FelicoResult, FelicoResultExt};
use crate::infra::shared_string::SharedString;
use crate::infra::source_file::SourceFileHandle;
use crate::interpret::core_definitions::TypeFactory;

#[derive(Debug)]
pub struct Parser {
    lexer: Lexer,
    current_token: Token,
    next_token: Token,
    source_file: SourceFileHandle,
    type_factory: TypeFactory,
}

impl Parser {
    pub fn new(source_file: SourceFileHandle, type_factory: &TypeFactory) -> FelicoResult<Self> {
        let mut lexer = Lexer::new(source_file.clone()).whatever_context("oops")?;
        let current_token = lexer
            .next()
            .ok_or_else(|| failed("Expected at least one token"))?;
        let next_token = lexer.next().unwrap_or(current_token.clone());
        Ok(Parser {
            lexer,
            current_token,
            next_token,
            source_file,
            type_factory: type_factory.clone(),
        })
    }

    pub fn new_in_memory(
        filename: &str,
        source_code: &str,
        type_factory: &TypeFactory,
    ) -> FelicoResult<Self> {
        Self::new(
            SourceFileHandle::from_string(filename, source_code),
            type_factory,
        )
    }

    pub fn advance(&mut self) {
        std::mem::swap(&mut self.current_token, &mut self.next_token);
        if let Some(token) = self.lexer.next() {
            self.next_token = token;
        } else {
            // EOF
            self.next_token = self.current_token.clone();
        }
    }

    pub fn parse_program(mut self) -> FelicoResult<AstNode<Program>> {
        let start_location = self.current_location();
        let mut stmts: Vec<AstNode<Stmt>> = vec![];
        while self.current_token.token_type != TokenType::EOF {
            stmts.push(self.parse_decl()?)
        }
        self.consume(TokenType::EOF, "Expected end of file")?;
        self.create_node(start_location, Program { stmts })
    }

    fn parse_decl(&mut self) -> FelicoResult<AstNode<Stmt>> {
        match self.current_token.token_type {
            TokenType::Let => {
                let node = self.parse_let_stmt()?;
                self.consume(TokenType::Semicolon, "Expected statement terminator (';')")?;
                Ok(node)
            }
            TokenType::Struct => {
                let node = self.parse_struct_stmt()?;
                Ok(node)
            }
            TokenType::Fun => {
                let node = self.parse_fun_stmt("function")?;
                Ok(node)
            }
            _ => self.parse_stmt(),
        }
    }

    fn parse_let_stmt(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::Let, "let expected")?;
        let name = self.consume(TokenType::Identifier, "Expected identifier after let")?;
        let type_expression = if self.is_at(TokenType::Colon) {
            self.advance();
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        self.consume(TokenType::Equal, "Expected '=' in let declaration")?;
        let expression = self.parse_expr()?;
        self.create_node(
            start_location,
            Stmt::Let(LetStmt {
                name,
                expression,
                type_expression,
            }),
        )
    }

    fn parse_separated<T>(
        &mut self,
        parse_fn: impl Fn(&mut Parser) -> FelicoResult<Option<T>>,
    ) -> FelicoResult<Vec<T>> {
        let mut result: Vec<T> = Vec::new();
        loop {
            if let Some(item) = parse_fn(self)? {
                result.push(item);
            } else {
                break;
            }
            if !self.is_at(TokenType::Comma) {
                break;
            }
            self.advance();
        }
        Ok(result)
    }

    fn parse_struct_stmt(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::Struct, "struct expected")?;
        let name = self.consume(TokenType::Identifier, "Expected identifier after struct")?;
        self.consume(TokenType::LeftBrace, "Expected '{'")?;
        let fields = self.parse_separated(|parser| {
            if !parser.is_at(TokenType::Identifier) {
                return Ok(None);
            }
            let field_start_location = parser.current_location();
            let field_name =
                parser.consume(TokenType::Identifier, "Expected field name in struct")?;
            parser.consume(TokenType::Colon, "Expected ':' after field name")?;
            let type_expression = parser.parse_type_expression()?;
            Ok(Some(parser.create_node(
                field_start_location,
                StructStmtField {
                    name: field_name,
                    type_expression,
                },
            )?))
        })?;
        self.consume(TokenType::RightBrace, "Expected '}' to complete class")?;
        self.create_node(start_location, Stmt::Struct(StructStmt { name, fields }))
    }

    fn parse_fun_stmt(&mut self, _kind: &str) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::Fun, "fun expected")?;
        let name = self.consume(TokenType::Identifier, "Expected function identifier")?;
        self.consume(TokenType::LeftParen, "Expected '('")?;
        let parameters = self.parse_separated(|parser| {
            if !parser.is_at(TokenType::Identifier) {
                return Ok(None);
            }
            let parameter_name =
                parser.consume(TokenType::Identifier, "Expected parameter identifier")?;
            parser.consume(TokenType::Colon, "Expected ':' after parameter name")?;
            let type_expression = parser.parse_type_expression()?;
            Ok(Some(FunParameter::new(parameter_name, type_expression)))
        })?;
        if parameters.len() > 255 {
            bail!("Too many parameters in function");
        }
        self.consume(
            TokenType::RightParen,
            "Expected ')' to close parameter list",
        )?;
        let return_type = if self.is_at(TokenType::Arrow) {
            self.advance();
            self.parse_type_expression()?
        } else {
            self.create_node(
                start_location.clone(),
                Expr::Variable(VarUse {
                    variable: Token {
                        token_type: TokenType::Identifier,
                        location: Location {
                            source_file: self.source_file.clone(),
                            start_byte: start_location.start_byte,
                            end_byte: start_location.end_byte,
                        },
                        value: Some(SharedString::from("unit")),
                    },
                    distance: 0,
                }),
            )?
        };
        let body = self.parse_block()?;
        self.create_node(
            start_location,
            Stmt::Fun(FunStmt {
                name,
                parameters,
                return_type,
                body,
            }),
        )
    }

    fn parse_stmt(&mut self) -> FelicoResult<AstNode<Stmt>> {
        match self.current_token.token_type {
            TokenType::LeftBrace => self.parse_block(),
            TokenType::If => self.parse_if(),
            TokenType::While => self.parse_while(),
            TokenType::For => self.parse_for(),
            TokenType::Return => self.parse_return(),
            _ => {
                let node = self.parse_expr_stmt()?;
                self.consume(TokenType::Semicolon, "Expected statement terminator (';')")?;
                Ok(node)
            }
        }
    }

    fn parse_return(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::Return, "return expected")?;
        let expression = if self.current_token.token_type != TokenType::Semicolon {
            self.parse_expr()?
        } else {
            self.create_node(start_location.clone(), Expr::Literal(LiteralExpr::Unit))?
        };
        self.consume(
            TokenType::Semicolon,
            "Expected semicolon after return statement",
        )?;
        self.create_node(start_location, Stmt::Return(ReturnStmt { expression }))
    }
    fn parse_expr_stmt(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        let expression = self.parse_expr()?;
        self.create_node(start_location, Stmt::Expression(ExprStmt { expression }))
    }

    fn parse_if(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::If, "Expected 'if'")?;
        self.consume(TokenType::LeftParen, "Expected '(' after if")?;
        let condition = self.parse_expr()?;
        self.consume(TokenType::RightParen, "Expected ')' after if condition")?;
        let then_stmt = self.parse_stmt()?;
        let else_stmt = if self.is_at(TokenType::Else) {
            self.advance();
            Some(self.parse_stmt()?)
        } else {
            None
        };
        self.create_node(
            start_location,
            Stmt::If(IfStmt {
                condition,
                then_stmt,
                else_stmt,
            }),
        )
    }

    fn parse_while(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::While, "Expected 'while'")?;
        self.consume(TokenType::LeftParen, "Expected '(' after while")?;
        let condition = self.parse_expr()?;
        self.consume(TokenType::RightParen, "Expected ')' after while condition")?;
        let body_stmt = self.parse_stmt()?;
        self.create_node(
            start_location,
            Stmt::While(WhileStmt {
                condition,
                body_stmt,
            }),
        )
    }

    fn parse_for(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::For, "Expected 'for'")?;
        self.consume(TokenType::LeftParen, "Expected '(' after for")?;
        let initializer = match self.current_token.token_type {
            TokenType::Let => Some(self.parse_let_stmt()?),
            TokenType::Semicolon => None,
            _ => Some(self.parse_expr_stmt()?),
        };
        self.consume(TokenType::Semicolon, "Expected ';' in for statement")?;
        let condition_location = self.current_location();
        let condition = match self.current_token.token_type {
            TokenType::Semicolon => {
                self.create_node(condition_location, Expr::Literal(LiteralExpr::Bool(true)))?
            }
            _ => self.parse_expr()?,
        };
        self.consume(TokenType::Semicolon, "Expected ';' in for statement")?;
        let increment = match self.current_token.token_type {
            TokenType::RightParen => None,
            _ => Some((self.current_location(), self.parse_expr()?)),
        };
        self.consume(TokenType::RightParen, "Expected ')' in for statement")?;
        let mut body_stmt = self.parse_stmt()?;
        if let Some((start, expression)) = increment {
            let increment_stmt =
                self.create_node(start, Stmt::Expression(ExprStmt { expression }))?;
            body_stmt = self.create_node(
                body_stmt.location.clone(),
                Stmt::Block(BlockStmt {
                    stmts: vec![body_stmt, increment_stmt],
                }),
            )?
        }
        let mut while_stmt = self.create_node(
            start_location.clone(),
            Stmt::While(WhileStmt {
                condition,
                body_stmt,
            }),
        )?;
        if let Some(initializer) = initializer {
            while_stmt = self.create_node(
                start_location,
                Stmt::Block(BlockStmt {
                    stmts: vec![initializer, while_stmt],
                }),
            )?
        }
        Ok(while_stmt)
    }

    pub fn parse_block(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::LeftBrace, "Expected left brace ('{')")?;
        let mut stmts: Vec<AstNode<Stmt>> = vec![];

        while self.current_token.token_type != TokenType::RightBrace {
            stmts.push(self.parse_decl()?)
        }
        self.consume(TokenType::RightBrace, "Expected right brace ('}')")?;
        self.create_node(start_location, Stmt::Block(BlockStmt { stmts }))
    }

    pub fn parse_expression(mut self) -> FelicoResult<AstNode<Expr>> {
        let result = self.parse_expr();
        self.consume(TokenType::EOF, "Expected end of input (EOF)")?;
        result
    }

    fn parse_expr(&mut self) -> FelicoResult<AstNode<Expr>> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let expr = self.parse_or()?;

        if self.is_at(TokenType::Equal) {
            self.advance();
            let value = self.parse_assignment()?;
            return if let Expr::Variable(var_use) = *expr.data {
                self.create_node(
                    start_location,
                    Expr::Assign(AssignExpr {
                        destination: var_use.variable,
                        value,
                        distance: -2000,
                    }),
                )
            } else if let Expr::Get(get) = *expr.data {
                self.create_node(
                    start_location,
                    Expr::Set(SetExpr {
                        value,
                        object: get.object,
                        name: get.name,
                    }),
                )
            } else {
                self.create_diagnostic("Invalid assignment target", |diagnostic| {
                    diagnostic.add_primary_label(&expr.location);
                    diagnostic.set_help(
                        "Assignment target must be an l-value (e.g. a variable or field)",
                    );
                })
            };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_and()?;
        while self.is_at(TokenType::Or) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_and()?;
            expr = self.create_node(
                start_location.clone(),
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_equality()?;
        while self.is_at(TokenType::And) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_equality()?;
            expr = self.create_node(
                start_location.clone(),
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }
    fn parse_equality(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_comparison()?;
        while self.is_at(TokenType::BangEqual) || self.is_at(TokenType::EqualEqual) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_comparison()?;
            expr = self.create_node(
                start_location.clone(),
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_term()?;
        while self.is_at(TokenType::Less)
            || self.is_at(TokenType::LessEqual)
            || self.is_at(TokenType::Greater)
            || self.is_at(TokenType::GreaterEqual)
        {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_term()?;
            expr = self.create_node(
                start_location.clone(),
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_factor()?;
        while self.is_at(TokenType::Plus) || self.is_at(TokenType::Minus) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_factor()?;
            expr = self.create_node(
                start_location.clone(),
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_unary()?;
        while self.is_at(TokenType::Star) || self.is_at(TokenType::Slash) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_unary()?;
            expr = self.create_node(
                start_location.clone(),
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_type_expression(&mut self) -> FelicoResult<AstNode<Expr>> {
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> FelicoResult<AstNode<Expr>> {
        let result = self.create_node(
            self.current_location(),
            Expr::Literal(match self.current_token.token_type {
                TokenType::Number => {
                    let number: f64 = self.current_token.lexeme().parse().map_err(|e| {
                        format!(
                            "Failed to parse number '{}': {}",
                            self.current_token.lexeme(),
                            e
                        )
                    })?;
                    LiteralExpr::F64(number)
                }
                TokenType::String => {
                    let lexeme = self.current_token.lexeme();
                    let string = lexeme[1..lexeme.len() - 1].to_string();
                    LiteralExpr::Str(string)
                }
                TokenType::True => LiteralExpr::Bool(true),
                TokenType::False => LiteralExpr::Bool(false),
                TokenType::Identifier => {
                    let result = self.create_node(
                        self.current_location(),
                        Expr::Variable(VarUse {
                            variable: self.current_token.clone(),
                            distance: -1000,
                        }),
                    );
                    self.advance();
                    return result;
                }
                TokenType::LeftParen => {
                    let start_location = self.current_location();
                    self.advance();
                    if self.is_at(TokenType::RightParen) {
                        // empty tuple
                        self.advance();
                        return self.create_node(self.current_location(), Expr::new_tuple(vec![]));
                    }
                    let mut components = self.parse_separated(|parser| {
                        if parser.is_at(TokenType::RightParen) {
                            return Ok(None);
                        }
                        Ok(Some(parser.parse_expr()?))
                    })?;
                    self.consume(TokenType::RightParen, "Expect closing ')' after expression")?;
                    if components.len() == 1 {
                        return Ok(components.pop().unwrap());
                    }
                    return self.create_node(start_location, Expr::new_tuple(components));
                }
                _ => {
                    return self.create_diagnostic(
                        format!(
                            "Unexpected token '{}' in expression",
                            self.current_token.token_type
                        ),
                        |diagnostic| {
                            diagnostic.add_primary_label(&self.current_token.location);
                        },
                    );
                }
            }),
        );
        self.advance();
        result
    }

    #[track_caller]
    fn consume(&mut self, expected_token_type: TokenType, msg: &str) -> FelicoResult<Token> {
        if self.is_at(expected_token_type) {
            let token = self.current_token.clone();
            self.advance();
            Ok(token)
        } else {
            self.create_diagnostic(
                format!("{}, found {} instead", msg, self.current_token),
                |diagnostic| diagnostic.add_primary_label(&self.current_token.location),
            )
        }
    }

    #[track_caller]
    fn create_diagnostic<T, S: Into<String>>(
        &self,
        message: S,
        mut f: impl FnMut(&mut InterpreterDiagnostic),
    ) -> FelicoResult<T> {
        let mut diagnostic = InterpreterDiagnostic::new(&self.source_file, message.into());
        f(&mut diagnostic);
        Err(diagnostic.into())
    }

    fn parse_unary(&mut self) -> FelicoResult<AstNode<Expr>> {
        match self.current_token.token_type {
            TokenType::Bang | TokenType::Minus => {
                let start_location = self.current_location();
                let operator = self.current_token.clone();
                self.advance();
                let right = self.parse_unary()?;
                self.create_node(start_location, Expr::Unary(UnaryExpr { operator, right }))
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> FelicoResult<AstNode<Expr>> {
        let mut expr = self.parse_primary()?;
        let start_location = self.current_location();
        loop {
            if self.is_at(TokenType::LeftParen) {
                self.advance();
                expr = self.finish_call(expr, start_location.clone())?;
            } else if self.is_at(TokenType::Dot) {
                self.advance();
                let name = self.consume(TokenType::Identifier, "Expected identifier after '.'")?;
                expr = self.create_node(
                    start_location.clone(),
                    Expr::Get(GetExpr { object: expr, name }),
                )?;
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn finish_call(
        &mut self,
        callee: AstNode<Expr>,
        start_location: Location,
    ) -> FelicoResult<AstNode<Expr>> {
        let arguments = self.parse_separated(|parser| {
            if parser.is_at(TokenType::RightParen) {
                return Ok(None);
            }
            Ok(Some(parser.parse_expr()?))
        })?;
        if arguments.len() >= 255 {
            bail!("Too many arguments in call expression");
        }
        self.consume(
            TokenType::RightParen,
            "Expected ')' after function call arguments",
        )?;
        self.create_node(start_location, Expr::Call(CallExpr { callee, arguments }))
    }

    fn create_node<T: AstData>(
        &mut self,
        start_location: Location,
        data: T,
    ) -> FelicoResult<AstNode<T>> {
        let start = start_location;
        let end = &self.current_token.location;
        let mut location = start.clone();
        if start.start_byte != end.end_byte {
            location.end_byte = end.end_byte;
        }
        Ok(AstNode::new(data, location, self.type_factory.unknown()))
    }

    fn current_location(&self) -> Location {
        self.current_token.location.clone()
    }

    #[inline]
    fn is_at(&self, token_type: TokenType) -> bool {
        self.current_token.token_type == token_type
    }
}

pub fn parse_expression(code_source: SourceFileHandle) -> FelicoResult<AstNode<Expr>> {
    let parser = Parser::new(code_source, &TypeFactory::new())?;
    parser.parse_expression()
}

pub fn parse_program(
    code_source: SourceFileHandle,
    type_factory: &TypeFactory,
) -> FelicoResult<AstNode<Program>> {
    let parser = Parser::new(code_source, type_factory)?;
    parser.parse_program()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::print_ast::{ast_to_string, AstPrinter};
    use crate::infra::diagnostic::unwrap_diagnostic_to_string;
    use expect_test::{expect, Expect};

    fn test_parse_expression(name: &str, input: &str, expected: Expect) {
        let parser = Parser::new_in_memory(name, input, &TypeFactory::new()).unwrap();
        let expr = parser.parse_expression().unwrap();
        AstPrinter::new().print_expr(&expr).unwrap();
        let printed_ast = AstPrinter::new().print_expr(&expr).unwrap();
        expected.assert_eq(&printed_ast);
    }

    macro_rules! test_expr {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_parse_expression(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_expr!(
        bool_true: "true" => expect![[r#"
            Bool(true)     [0+4]
        "#]];
        bool_false: "false" => expect![[r#"
            Bool(false)     [0+5]
        "#]];

        nil: "nil" => expect![[r#"
            Read 'nil'     [0+3]
        "#]];

        string_empty: "\"\"" => expect![[r#"
            Str("")     [0+2]
        "#]];
        string_space: "\" \"" => expect![[r#"
            Str(" ")     [0+3]
        "#]];
        string_newline: "\"\n\"" => expect![[r#"
            Str("\n")     [0+3]
        "#]];
        string_foo: "\"foo\"" => expect![[r#"
            Str("foo")     [0+5]
        "#]];
        string_unicode: "\"😶‍🌫️\"" => expect![[r#"
            Str("😶\u{200d}🌫\u{fe0f}")     [0+16]
        "#]];

        identifier_foo: "foo" => expect![[r#"
            Read 'foo'     [0+3]
        "#]];
        identifier_underscore_foo: "_foo" => expect![[r#"
            Read '_foo'     [0+4]
        "#]];
        identifier_uppercase: "Uppercase" => expect![[r#"
            Read 'Uppercase'     [0+9]
        "#]];
        identifier_allcaps: "ALL_CAPS_" => expect![[r#"
            Read 'ALL_CAPS_'     [0+9]
        "#]];


        number_literal_0: "0" => expect![[r#"
            F64(0.0)     [0+1]
        "#]];
        number_literal_123: "123" => expect![[r#"
            F64(123.0)     [0+3]
        "#]];
        number_literal_3_141: "3.141" => expect![[r#"
            F64(3.141)     [0+5]
        "#]];
        number_literal_minus_1: "-1.0" => expect![[r#"
            -     [0+4]
            └── F64(1.0)     [1+3]
        "#]];
        expression_equal_equal: "1.0==2.0" => expect![[r#"
            ==     [0+8]
            ├── F64(1.0)     [0+3]
            └── F64(2.0)     [5+3]
        "#]];
        bang_equal_equal: "-1 == - 10" => expect![[r#"
            ==     [0+10]
            ├── -     [0+5]
            │   └── F64(1.0)     [1+1]
            └── -     [6+4]
                └── F64(10.0)     [8+2]
        "#]];
        expression_less: "1<2" => expect![[r#"
            <     [0+3]
            ├── F64(1.0)     [0+1]
            └── F64(2.0)     [2+1]
        "#]];
        expression_less_equal: "1<=2" => expect![[r#"
            <=     [0+4]
            ├── F64(1.0)     [0+1]
            └── F64(2.0)     [3+1]
        "#]];
        expression_greater: "1>2" => expect![[r#"
            >     [0+3]
            ├── F64(1.0)     [0+1]
            └── F64(2.0)     [2+1]
        "#]];
        expression_greater_equal: "1>=2" => expect![[r#"
            >=     [0+4]
            ├── F64(1.0)     [0+1]
            └── F64(2.0)     [3+1]
        "#]];
        expression_precedence: "true == 1>=2" => expect![[r#"
            ==     [0+12]
            ├── Bool(true)     [0+4]
            └── >=     [8+4]
                ├── F64(1.0)     [8+1]
                └── F64(2.0)     [11+1]
        "#]];
        expression_plus: "1+2" => expect![[r#"
            +     [0+3]
            ├── F64(1.0)     [0+1]
            └── F64(2.0)     [2+1]
        "#]];
        expression_minus: "1-2" => expect![[r#"
            -     [0+3]
            ├── F64(1.0)     [0+1]
            └── F64(2.0)     [2+1]
        "#]];
        expression_times: "1*2" => expect![[r#"
            *     [0+3]
            ├── F64(1.0)     [0+1]
            └── F64(2.0)     [2+1]
        "#]];
        expression_division: "1/2" => expect![[r#"
            /     [0+3]
            ├── F64(1.0)     [0+1]
            └── F64(2.0)     [2+1]
        "#]];
        expression_precedence_math: "4==1+2*-3" => expect![[r#"
            ==     [0+9]
            ├── F64(4.0)     [0+1]
            └── +     [3+6]
                ├── F64(1.0)     [3+1]
                └── *     [5+4]
                    ├── F64(2.0)     [5+1]
                    └── -     [7+2]
                        └── F64(3.0)     [8+1]
        "#]];
        expression_paren_simple: "(1)" => expect![[r#"
            F64(1.0)     [1+1]
        "#]];
        expression_paren_nexted: "((1))" => expect![[r#"
            F64(1.0)     [2+1]
        "#]];
        expression_paren_complex: "3*(1+2)" => expect![[r#"
            *     [0+7]
            ├── F64(3.0)     [0+1]
            └── +     [3+4]
                ├── F64(1.0)     [3+1]
                └── F64(2.0)     [5+1]
        "#]];
        expression_assign: "a=2" => expect![[r#"
            'a' (Identifier) =      [0+3]
            └── F64(2.0)     [2+1]
        "#]];
        expression_assign_twice: "a=b=3" => expect![[r#"
            'a' (Identifier) =      [0+5]
            └── 'b' (Identifier) =      [2+3]
                └── F64(3.0)     [4+1]
        "#]];
        expression_assign_twice2: "a=b=3" => expect![[r#"
            'a' (Identifier) =      [0+5]
            └── 'b' (Identifier) =      [2+3]
                └── F64(3.0)     [4+1]
        "#]];
        expression_and: "a && b" => expect![[r#"
            &&     [0+6]
            ├── Read 'a'     [0+1]
            └── Read 'b'     [5+1]
        "#]];
        expression_or: "a || b" => expect![[r#"
            ||     [0+6]
            ├── Read 'a'     [0+1]
            └── Read 'b'     [5+1]
        "#]];
        expression_and_or: "a && b || c" => expect![[r#"
            ||     [0+11]
            ├── &&     [0+9]
            │   ├── Read 'a'     [0+1]
            │   └── Read 'b'     [5+1]
            └── Read 'c'     [10+1]
        "#]];
        expression_or_and: "a || b && c" => expect![[r#"
            ||     [0+11]
            ├── Read 'a'     [0+1]
            └── &&     [5+6]
                ├── Read 'b'     [5+1]
                └── Read 'c'     [10+1]
        "#]];
        expression_call_empty: "foo()" => expect![[r#"
            Call     [3+2]
            └── Read 'foo'     [0+3]
        "#]];
        expression_call_one_arg: "foo(bar)" => expect![[r#"
            Call     [3+5]
            ├── Read 'foo'     [0+3]
            └── Read 'bar'     [4+3]
        "#]];
        expression_call_two_args: "foo(bar,baz)" => expect![[r#"
            Call     [3+9]
            ├── Read 'foo'     [0+3]
            ├── Read 'bar'     [4+3]
            └── Read 'baz'     [8+3]
        "#]];
        expression_call_with_trailing_comma: "foo(bar,baz,)" => expect![[r#"
            Call     [3+10]
            ├── Read 'foo'     [0+3]
            ├── Read 'bar'     [4+3]
            └── Read 'baz'     [8+3]
        "#]];
        expression_call_twice: "foo()()" => expect![[r#"
            Call     [3+4]
            └── Call     [3+3]
                └── Read 'foo'     [0+3]
        "#]];

        tuple_empty: "()" => expect![[r#"
            Tuple     [2+0]
        "#]];

        tuple_pair: "(3, true)" => expect![[r#"
            Tuple     [0+9]
            ├── F64(3.0)     [1+1]
            └── Bool(true)     [4+4]
        "#]];
    );

    fn test_parse_program(name: &str, input: &str, expected: Expect) {
        let parser = Parser::new_in_memory(name, input, &TypeFactory::new()).unwrap();
        let program = parser.parse_program().unwrap();
        let printed_ast = ast_to_string(&program).unwrap();

        expected.assert_eq(&printed_ast);
    }

    macro_rules! test_program {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_parse_program(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_program!(
                program_empty: "" => expect![[r#"
                    Program
                "#]];
                program_bool_true: "true;" => expect![[r#"
                    Program
                    └── Bool(true)     [0+4]     [0+5]
                "#]];
                program_addition: "1+2;" => expect![[r#"
                    Program
                    └── +     [0+4]     [0+4]
                        ├── F64(1.0)     [0+1]
                        └── F64(2.0)     [2+1]
                "#]];
                program_multiline: "\"Hello\";\n\"World\";" => expect![[r#"
                    Program
                    ├── Str("Hello")     [0+7]     [0+8]
                    └── Str("World")     [9+7]     [9+8]
                "#]];

                program_true: "true;" => expect![[r#"
                    Program
                    └── Bool(true)     [0+4]     [0+5]
                "#]];
                program_string_addition: "\"Hello \" + 3;" => expect![[r#"
                    Program
                    └── +     [0+13]     [0+13]
                        ├── Str("Hello ")     [0+8]
                        └── F64(3.0)     [11+1]
                "#]];
                program_let_decl: "let a = false;" => expect![[r#"
                    Program
                    └── Let ''a' (Identifier)'     [0+14]
                        └── Bool(false)     [8+5]
                "#]];
                program_let_decl_with_type: "let a: bool = false;" => expect![[r#"
                    Program
                    └── Let ''a' (Identifier)'     [0+20]
                        └── Bool(false)     [14+5]
                "#]];
                program_program: "let a = 1;let b = a+a;b;" => expect![[r#"
                    Program
                    ├── Let ''a' (Identifier)'     [0+10]
                    │   └── F64(1.0)     [8+1]
                    ├── Let ''b' (Identifier)'     [10+12]
                    │   └── +     [18+4]
                    │       ├── Read 'a'     [18+1]
                    │       └── Read 'a'     [20+1]
                    └── Read 'b'     [22+1]     [22+2]
                "#]];

                program_assign: "a=1;" => expect![[r#"
                    Program
                    └── 'a' (Identifier) =      [0+4]     [0+4]
                        └── F64(1.0)     [2+1]
                "#]];
                program_assign_twice2: "a=b=3;" => expect![[r#"
                    Program
                    └── 'a' (Identifier) =      [0+6]     [0+6]
                        └── 'b' (Identifier) =      [2+4]
                            └── F64(3.0)     [4+1]
                "#]];
                program_block_empty: "{}" => expect![[r#"
                    Program
                    └── Block     [0+2]
                "#]];
                program_nested_block: "{{foo;}}" => expect![[r#"
                    Program
                    └── Block     [0+8]
                        └── Block     [1+7]
                            └── Read 'foo'     [2+3]     [2+4]
                "#]];

               program_if: "if(c) a;" => expect![[r#"
                   Program
                   └── If     [0+8]
                       ├── Read 'c'     [3+1]
                       └── Read 'a'     [6+1]     [6+2]
               "#]];
               program_if_else: "if(c) a; else b;" => expect![[r#"
                   Program
                   └── If     [0+16]
                       ├── Read 'c'     [3+1]
                       ├── Read 'a'     [6+1]     [6+2]
                       └── Read 'b'     [14+1]     [14+2]
               "#]];

               program_while: "while(a) b;" => expect![[r#"
                   Program
                   └── While     [0+11]
                       ├── Read 'a'     [6+1]
                       └── Read 'b'     [9+1]     [9+2]
               "#]];

               program_for_let: "for(let i = 1; i < 3; i = i + 1) i;" => expect![[r#"
                   Program
                   └── Block     [0+35]
                       ├── Let ''i' (Identifier)'     [4+10]
                       │   └── F64(1.0)     [12+1]
                       └── While     [0+35]
                           ├── <     [15+6]
                           │   ├── Read 'i'     [15+1]
                           │   └── F64(3.0)     [19+1]
                           └── Block     [33+2]
                               ├── Read 'i'     [33+1]     [33+2]
                               └── 'i' (Identifier) =      [22+10]     [22+13]
                                   └── +     [26+6]
                                       ├── Read 'i'     [26+1]
                                       └── F64(1.0)     [30+1]
               "#]];
               program_for_empty: "for(;;) i;" => expect![[r#"
                   Program
                   └── While     [0+10]
                       ├── Bool(true)     [5+1]
                       └── Read 'i'     [8+1]     [8+2]
               "#]];

               program_fun_empty: "fun foo() {}" => expect![[r#"
                   Program
                   └── Declare fun 'foo()'     [0+12]
                       └── Return type: Read 'unit'     [0+11]
               "#]];
               program_fun_simple: "fun foo(a: bool) {a;} " => expect![[r#"
                   Program
                   └── Declare fun 'foo(a)'     [0+22]
                       ├── Param a
                       │   └── Read 'bool'     [11+4]
                       ├── Return type: Read 'unit'     [0+18]
                       └── Read 'a'     [18+1]     [18+2]
               "#]];
               program_fun_with_return_type: "fun not(a: bool,) -> bool {return !a;} " => expect![[r#"
                   Program
                   └── Declare fun 'not(a)'     [0+39]
                       ├── Param a
                       │   └── Read 'bool'     [11+4]
                       ├── Return type: Read 'bool'     [21+4]
                       └── Return     [27+11]
                           └── !     [34+3]
                               └── Read 'a'     [35+1]
               "#]];
               program_fun_return: "fun nop() {return;} " => expect![[r#"
                   Program
                   └── Declare fun 'nop()'     [0+20]
                       ├── Return type: Read 'unit'     [0+11]
                       └── Return     [11+8]
                           └── Unit     [11+7]
               "#]];
               program_fun_return_value: "fun three(a: bool) {return 3;} " => expect![[r#"
                   Program
                   └── Declare fun 'three(a)'     [0+31]
                       ├── Param a
                       │   └── Read 'bool'     [13+4]
                       ├── Return type: Read 'unit'     [0+20]
                       └── Return     [20+10]
                           └── F64(3.0)     [27+1]
               "#]];
               program_fun_return_expression: "fun twice(a: f64) {return a+a;} " => expect![[r#"
                   Program
                   └── Declare fun 'twice(a)'     [0+32]
                       ├── Param a
                       │   └── Read 'f64'     [13+3]
                       ├── Return type: Read 'unit'     [0+19]
                       └── Return     [19+12]
                           └── +     [26+4]
                               ├── Read 'a'     [26+1]
                               └── Read 'a'     [28+1]
               "#]];
               program_property_access: "a.b;" => expect![[r#"
                   Program
                   └── Get b     [1+3]     [0+4]
                       └── Read 'a'     [0+1]
               "#]];
               program_property_set: "a.b = 3;" => expect![[r#"
                   Program
                   └── Set b     [0+8]     [0+8]
                       ├── Read 'a'     [0+1]
                       └── F64(3.0)     [6+1]
               "#]];
               program_struct_simple: "
               struct Foo {
                    bar: bool,
                    baz: f64
                }
               " => expect![[r#"
                   Program
                   └── Struct 'Foo'     [16+106]
                       ├── Field bar     [49+10]
                       │   └── Read 'bool'     [54+4]
                       └── Field baz     [80+26]
                           └── Read 'f64'     [85+3]
               "#]];
               program_struct_trailing_comma: "
               struct Foo {
                    bar: bool,
                    baz: f64,
                }
               " => expect![[r#"
                   Program
                   └── Struct 'Foo'     [16+107]
                       ├── Field bar     [49+10]
                       │   └── Read 'bool'     [54+4]
                       └── Field baz     [80+9]
                           └── Read 'f64'     [85+3]
               "#]];
                program_struct_empty: "
               struct Empty {}
               " => expect![[r#"
                   Program
                   └── Struct 'Empty'     [16+31]
               "#]];

    );

    fn test_parse_program_error(name: &str, input: &str, expected: Expect) {
        let parser = Parser::new_in_memory(name, input, &TypeFactory::new()).unwrap();
        let result = parser.parse_program();
        let diagnostic_string = unwrap_diagnostic_to_string(&result);
        expected.assert_eq(&diagnostic_string);
    }

    macro_rules! test_parse_error {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_parse_program_error(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_parse_error!(
        naked_if_condition: "if x" => expect![[r#"
            × Expected '(' after if, found 'x' (Identifier) instead
               ╭─[naked_if_condition:1:4]
             1 │ if x
               ·    ─
               ╰────"#]];
        unclosed_parens: "(3 4" => expect![[r#"
            × Expect closing ')' after expression, found '4' (Number) instead
               ╭─[unclosed_parens:1:4]
             1 │ (3 4
               ·    ─
               ╰────"#]];
         invalid_assignment: "3 = true" => expect![[r#"
             × Invalid assignment target
                ╭─[invalid_assignment:1:1]
              1 │ 3 = true
                · ─
                ╰────
               help: Assignment target must be an l-value (e.g. a variable or field)"#]];
         unexpected_expression_part: "3 + if" => expect![[r#"
             × Unexpected token 'If' in expression
                ╭─[unexpected_expression_part:1:5]
              1 │ 3 + if
                ·     ──
                ╰────"#]];
         incomplete_statement: "print true" => expect![[r#"
             × Expected statement terminator (';'), found 'true' (True) instead
                ╭─[incomplete_statement:1:7]
              1 │ print true
                ·       ────
                ╰────"#]];
         chained_values: "true \"foo\"" => expect![[r#"
             × Expected statement terminator (';'), found '"foo"' (String) instead
                ╭─[chained_values:1:6]
              1 │ true "foo"
                ·      ─────
                ╰────"#]];
    );
}
