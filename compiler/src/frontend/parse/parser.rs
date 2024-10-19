use crate::frontend::ast::expr::{
    AssignExpr, BinaryExpr, BlockExpr, CallExpr, CreateStructExpr, CreateStructInitializer, Expr,
    GetExpr, IfExpr, LiteralExpr, ReturnExpr, SetExpr, UnaryExpr, VarUse,
};
use crate::frontend::ast::module::Module;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::qualified_name::QualifiedName;
use crate::frontend::ast::stmt::{
    ExprStmt, FunParameter, FunStmt, ImplStmt, LetStmt, Stmt, StructStmt, StructStmtField,
    TraitStmt, WhileStmt,
};
use crate::frontend::ast::AstData;
use crate::frontend::lex::lexer::Lexer;
use crate::frontend::lex::token::{Token, TokenType};
use crate::infra::diagnostic::InterpreterDiagnostic;
use crate::infra::full_name::FullName;
use crate::infra::result::{bail, failed, FelicoResult, FelicoResultExt};
use crate::infra::shared_string::SharedString;
use crate::infra::source_file::SourceFile;
use crate::infra::source_span::SourceSpan;
use crate::model::type_factory::TypeFactory;
use crate::model::workspace::Workspace;

#[derive(Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token<'a>,
    next_token: Token<'a>,
    source_file: SourceFile<'a>,
    type_factory: TypeFactory<'a>,
    module_name: FullName,
}

impl<'a> Parser<'a> {
    pub fn new(source_file: SourceFile<'a>, type_factory: TypeFactory<'a>) -> FelicoResult<Self> {
        let mut lexer = Lexer::new(source_file.clone()).whatever_context("oops")?;
        let current_token = lexer
            .next()
            .ok_or_else(|| failed("Expected at least one token"))?;
        let next_token = lexer.next().unwrap_or(current_token.clone());
        let file_path = source_file.filename();
        let file_name = file_path
            .rsplit_once("/")
            .map(|(_prefix, file_name)| file_name)
            .unwrap_or(file_path);
        let module_name = file_name.split_once(".").map(|a| a.0).unwrap_or(file_name);
        Ok(Parser {
            module_name: module_name.into(),
            lexer,
            current_token,
            next_token,
            source_file,
            type_factory,
        })
    }
    /*
        pub fn new_in_memory(
            filename: &str,
            source_code: &str,
            type_factory: TypeFactory<'a>,
        ) -> FelicoResult<Self> {
            Self::new(SourceFile::from_string(filename, source_code), type_factory)
        }
    */
    pub fn advance(&mut self) {
        std::mem::swap(&mut self.current_token, &mut self.next_token);
        if let Some(token) = self.lexer.next() {
            self.next_token = token;
        } else {
            // EOF
            self.next_token = self.current_token.clone();
        }
    }

    pub fn parse_script(&mut self) -> FelicoResult<AstNode<'a, Module<'a>>> {
        let start_location = self.current_location();

        let module = self.parse_module()?;

        // Create artificial main function
        let return_type = self.create_unit_var_use(&start_location)?;
        let result_expression =
            self.create_node(&start_location, Expr::Literal(LiteralExpr::Unit))?;
        let body = self.create_node(
            &start_location,
            Expr::Block(BlockExpr {
                stmts: module.data.stmts,
                result_expression,
            }),
        )?;

        let main_stmt = self.create_node(
            &start_location,
            Stmt::Fun(FunStmt {
                name: Token {
                    token_type: TokenType::Identifier,
                    location: start_location.clone(),
                    value: Some(SharedString::from("main")),
                },
                parameters: vec![],
                return_type,
                body,
            }),
        )?;
        let name = self.module_name.clone();
        self.create_node(
            &start_location,
            Module {
                name,
                stmts: vec![main_stmt],
            },
        )
    }

    pub fn parse_module(&mut self) -> FelicoResult<AstNode<'a, Module<'a>>> {
        let start_location = self.current_location();

        let mut stmts: Vec<AstNode<Stmt>> = vec![];
        while self.current_token.token_type != TokenType::EOF {
            if self.is_at(TokenType::Semicolon) {
                self.advance();
                continue;
            }
            stmts.push(self.parse_decl()?)
        }
        self.consume(TokenType::EOF, "Expected end of file")?;

        self.create_node(
            &start_location,
            Module {
                name: self.module_name.clone(),
                stmts,
            },
        )
    }

    fn parse_decl(&mut self) -> FelicoResult<AstNode<'a, Stmt<'a>>> {
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
            TokenType::Trait => {
                let node = self.parse_trait_stmt()?;
                Ok(node)
            }
            TokenType::Impl => {
                let node = self.parse_impl_stmt()?;
                Ok(node)
            }
            TokenType::Fun => {
                let node = self.parse_fun_stmt("function")?;
                Ok(node)
            }
            _ => self.parse_stmt(),
        }
    }

    fn parse_let_stmt(&mut self) -> FelicoResult<AstNode<'a, Stmt<'a>>> {
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
            &start_location,
            Stmt::Let(LetStmt {
                name,
                expression,
                type_expression,
            }),
        )
    }

    fn parse_separated<T: 'a>(
        &mut self,
        parse_fn: impl Fn(&mut Parser<'a>) -> FelicoResult<Option<T>>,
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

    fn parse_struct_stmt(&mut self) -> FelicoResult<AstNode<'a, Stmt<'a>>> {
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
                &field_start_location,
                StructStmtField {
                    name: field_name,
                    type_expression,
                },
            )?))
        })?;
        self.consume(TokenType::RightBrace, "Expected '}' to complete class")?;
        self.create_node(&start_location, Stmt::Struct(StructStmt { name, fields }))
    }

    fn parse_trait_stmt(&mut self) -> FelicoResult<AstNode<'a, Stmt<'a>>> {
        let start_location = self.current_location();
        self.consume(TokenType::Trait, "trait expected")?;
        let name = self.consume(TokenType::Identifier, "Expected identifier after trait")?;
        self.consume(TokenType::LeftBrace, "Expected '{'")?;
        self.consume(TokenType::RightBrace, "Expected '}' to complete trait")?;
        self.create_node(&start_location, Stmt::Trait(TraitStmt { name }))
    }

    fn parse_impl_stmt(&mut self) -> FelicoResult<AstNode<'a, Stmt<'a>>> {
        let start_location = self.current_location();
        self.consume(TokenType::Impl, "impl expected")?;
        let name = self.consume(TokenType::Identifier, "Expected identifier after impl")?;
        self.consume(TokenType::LeftBrace, "Expected '{'")?;
        let mut methods = vec![];
        loop {
            if self.is_at(TokenType::RightBrace) {
                break;
            }
            let stmt = self.parse_fun_stmt("method")?;
            let AstNode { location, data, ty } = stmt;
            let stmt = *data;
            let Stmt::Fun(fun_stmt) = stmt else {
                return Err(InterpreterDiagnostic::new(
                    &location,
                    format!(
                        "Expected function definition in impl block, found {:?} instead",
                        stmt
                    ),
                )
                .into());
            };
            methods.push(AstNode::new(fun_stmt, location, ty));
        }
        self.consume(TokenType::RightBrace, "Expected '}' to complete impl")?;
        self.create_node(&start_location, Stmt::Impl(ImplStmt { name, methods }))
    }

    fn parse_fun_stmt(&mut self, _kind: &str) -> FelicoResult<AstNode<'a, Stmt<'a>>> {
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
            self.create_unit_var_use(&start_location)?
        };
        let body = self.parse_block_expr()?;
        self.create_node(
            &start_location,
            Stmt::Fun(FunStmt {
                name,
                parameters,
                return_type,
                body,
            }),
        )
    }

    fn create_unit_var_use(
        &mut self,
        start_location: &SourceSpan<'a>,
    ) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        self.create_node(
            start_location,
            Expr::Variable(VarUse {
                name: AstNode::new(
                    QualifiedName {
                        parts: vec![Token {
                            token_type: TokenType::Identifier,
                            location: SourceSpan {
                                source_file: self.source_file.clone(),
                                start_byte: start_location.start_byte,
                                end_byte: start_location.end_byte,
                            },
                            value: Some(SharedString::from("unit")),
                        }],
                    },
                    SourceSpan::ephemeral(),
                    self.type_factory.unknown(),
                ),
                distance: 0,
            }),
        )
    }

    fn parse_stmt(&mut self) -> FelicoResult<AstNode<'a, Stmt<'a>>> {
        match self.current_token.token_type {
            TokenType::While => self.parse_while(),
            //            TokenType::For => self.parse_for(),
            _ => {
                let node = self.parse_expr_stmt()?;
                if let Stmt::Expression(expression) = &*node.data {
                    if !(matches!(*expression.expression.data, Expr::Block(_))
                        || matches!(*expression.expression.data, Expr::If(_)))
                    {
                        self.consume(TokenType::Semicolon, "Expected statement terminator (';')")?;
                    };
                } else {
                    bail!("Expected expression statement")
                }
                Ok(node)
            }
        }
    }

    fn parse_return(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        self.consume(TokenType::Return, "return expected")?;
        let expression = if self.current_token.token_type != TokenType::Semicolon {
            self.parse_expr()?
        } else {
            self.create_node(&start_location, Expr::Literal(LiteralExpr::Unit))?
        };
        self.create_node(&start_location, Expr::Return(ReturnExpr { expression }))
    }
    fn parse_expr_stmt(&mut self) -> FelicoResult<AstNode<'a, Stmt<'a>>> {
        let start_location = self.current_location();
        let expression = self.parse_expr()?;
        self.create_node(&start_location, Stmt::Expression(ExprStmt { expression }))
    }

    fn parse_if(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        self.consume(TokenType::If, "Expected 'if'")?;
        self.consume(TokenType::LeftParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expr()?;
        self.consume(TokenType::RightParen, "Expected ')' after 'if' condition")?;
        let then_expr = self.parse_expr()?;
        let else_expr = if self.is_at(TokenType::Else) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.create_node(
            &start_location,
            Expr::If(IfExpr {
                condition,
                then_expr,
                else_expr,
            }),
        )
    }

    fn parse_while(&mut self) -> FelicoResult<AstNode<'a, Stmt<'a>>> {
        let start_location = self.current_location();
        self.consume(TokenType::While, "Expected 'while'")?;
        self.consume(TokenType::LeftParen, "Expected '(' after while")?;
        let condition = self.parse_expr()?;
        self.consume(TokenType::RightParen, "Expected ')' after while condition")?;
        let body_stmt = self.parse_stmt()?;
        self.create_node(
            &start_location,
            Stmt::While(WhileStmt {
                condition,
                body_stmt,
            }),
        )
    }
    /*
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
                    Stmt::Expression(ExprStmt {})Block(BlockExpr {
                        stmts: vec![body_stmt, increment_stmt],
                    }),
                )?
            }
            let mut while_stmt = self.create_node(&start_location,
                Stmt::While(WhileStmt {
                    condition,
                    body_stmt,
                }),
            )?;
            if let Some(initializer) = initializer {
                while_stmt = self.create_node(&start_location,
                    Stmt::Block(BlockExpr {
                        stmts: vec![initializer, while_stmt],
                    }),
                )?
            }
            Ok(while_stmt)
        }
    */
    pub fn parse_block_expr(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        self.consume(TokenType::LeftBrace, "Expected left brace ('{')")?;
        let mut stmts: Vec<AstNode<Stmt>> = vec![];

        while self.current_token.token_type != TokenType::RightBrace {
            if self.is_at(TokenType::Semicolon) {
                self.advance();
                continue;
            }
            stmts.push(self.parse_decl()?)
        }
        // TODO: handle result expression
        let result_location = self.current_location();
        let result_expression =
            self.create_node(&result_location, Expr::Literal(LiteralExpr::Unit))?;

        self.consume(TokenType::RightBrace, "Expected right brace ('}')")?;
        self.create_node(
            &start_location,
            Expr::Block(BlockExpr {
                stmts,
                result_expression,
            }),
        )
    }

    pub fn parse_expression(mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let result = self.parse_expr();
        self.consume(TokenType::EOF, "Expected end of input (EOF)")?;
        result
    }

    fn parse_expr(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        let expr = self.parse_or()?;

        if self.is_at(TokenType::Equal) {
            self.advance();
            let value = self.parse_assignment()?;
            return if let Expr::Variable(var_use) = *expr.data {
                self.create_node(
                    &start_location,
                    Expr::Assign(AssignExpr {
                        destination: var_use.name,
                        value,
                        distance: -2000,
                    }),
                )
            } else if let Expr::Get(get) = *expr.data {
                self.create_node(
                    &start_location,
                    Expr::Set(SetExpr {
                        value,
                        object: get.object,
                        name: get.name,
                    }),
                )
            } else {
                return Err(InterpreterDiagnostic::new_with(
                    &expr.location,
                    format!("{:?} is not a valid assignment target", expr.data),
                    |diagnostic| {
                        diagnostic.set_help(
                            "Assignment target must be an l-value (e.g. a variable or field)",
                        );
                    },
                )
                .into());
            };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        let mut expr = self.parse_and()?;
        while self.is_at(TokenType::Or) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_and()?;
            expr = self.create_node(
                &start_location,
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        let mut expr = self.parse_equality()?;
        while self.is_at(TokenType::And) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_equality()?;
            expr = self.create_node(
                &start_location,
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }
    fn parse_equality(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        let mut expr = self.parse_comparison()?;
        while self.is_at(TokenType::BangEqual) || self.is_at(TokenType::EqualEqual) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_comparison()?;
            expr = self.create_node(
                &start_location,
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
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
                &start_location,
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        let mut expr = self.parse_factor()?;
        while self.is_at(TokenType::Plus) || self.is_at(TokenType::Minus) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_factor()?;
            expr = self.create_node(
                &start_location,
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        let mut expr = self.parse_unary()?;
        while self.is_at(TokenType::Star) || self.is_at(TokenType::Slash) {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_unary()?;
            expr = self.create_node(
                &start_location,
                Expr::Binary(BinaryExpr {
                    operator,
                    left: expr,
                    right,
                }),
            )?
        }
        Ok(expr)
    }

    fn parse_type_expression(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        self.parse_primary()
    }

    fn parse_create_struct(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        let primary = self.parse_primary()?;
        if !self.is_at(TokenType::LeftBrace) {
            return Ok(primary);
        }
        self.consume(
            TokenType::LeftBrace,
            "Expected '{' after struct constructor",
        )?;
        let field_initializers = self.parse_separated(|parser| {
            if !parser.is_at(TokenType::Identifier) {
                return Ok(None);
            }
            let field_name =
                parser.consume(TokenType::Identifier, "Expected field name in struct")?;
            parser.consume(TokenType::Colon, "Expected ':' after field name")?;
            let expression = parser.parse_expr()?;
            Ok(Some(CreateStructInitializer {
                field_name,
                expression,
            }))
        })?;
        self.consume(
            TokenType::RightBrace,
            "Expected '}' to complete struct instance creation",
        )?;
        self.create_node(
            &start_location,
            Expr::CreateStruct(CreateStructExpr {
                type_expression: primary,
                field_initializers,
            }),
        )
    }

    fn parse_primary(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let result = self.create_node(
            &self.current_location(),
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
                    let start_location = self.current_location();
                    let qualified_name = self.parse_qualified_name()?;
                    let location = qualified_name.location.clone();
                    let mut result = self.create_node(
                        &start_location,
                        Expr::Variable(VarUse {
                            name: qualified_name,
                            distance: -1000,
                        }),
                    )?;
                    result.location = location;
                    return Ok(result);
                }
                TokenType::LeftParen => {
                    self.advance();
                    let expression = self.parse_expr()?;
                    self.consume(TokenType::RightParen, "Expect closing ')' after expression")?;
                    return Ok(expression);
                }
                TokenType::LeftBrace => return self.parse_block_expr(),
                TokenType::If => return self.parse_if(),
                TokenType::Return => return self.parse_return(),
                _ => {
                    return Err(InterpreterDiagnostic::new(
                        &self.current_token.location,
                        format!(
                            "Unexpected token '{}' in expression",
                            self.current_token.token_type
                        ),
                    )
                    .into());
                }
            }),
        );
        self.advance();
        result
    }

    fn parse_qualified_name(&mut self) -> FelicoResult<AstNode<'a, QualifiedName<'a>>> {
        let start_location = self.current_location();
        let token = self.consume(TokenType::Identifier, "expected identifier")?;
        let mut last_location = token.location.clone();
        let mut parts = vec![token];
        while self.is_at(TokenType::ColonColon) {
            self.advance();
            let token = self.consume(
                TokenType::Identifier,
                "expected identifier after '::' in qualified name",
            )?;
            last_location = token.location.clone();
            parts.push(token);
        }
        let mut result = self.create_node(&start_location, QualifiedName { parts })?;
        result.location.end_byte = last_location.end_byte;
        Ok(result)
    }

    #[track_caller]
    fn consume(&mut self, expected_token_type: TokenType, msg: &str) -> FelicoResult<Token<'a>> {
        if self.is_at(expected_token_type) {
            let token = self.current_token.clone();
            self.advance();
            Ok(token)
        } else {
            Err(InterpreterDiagnostic::new(
                &self.current_location(),
                format!("{}, found {} instead", msg, self.current_token),
            )
            .into())
        }
    }

    fn parse_unary(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        match self.current_token.token_type {
            TokenType::Bang | TokenType::Minus => {
                let start_location = self.current_location();
                let operator = self.current_token.clone();
                self.advance();
                let right = self.parse_unary()?;
                self.create_node(&start_location, Expr::Unary(UnaryExpr { operator, right }))
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> FelicoResult<AstNode<'a, Expr<'a>>> {
        let start_location = self.current_location();
        let mut expr = self.parse_create_struct()?;
        loop {
            if self.is_at(TokenType::LeftParen) {
                self.advance();
                expr = self.finish_call(expr, start_location.clone())?;
            } else if self.is_at(TokenType::Dot) {
                self.advance();
                let name = self.consume(TokenType::Identifier, "Expected identifier after '.'")?;
                expr =
                    self.create_node(&start_location, Expr::Get(GetExpr { object: expr, name }))?;
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn finish_call(
        &mut self,
        callee: AstNode<'a, Expr<'a>>,
        start_location: SourceSpan<'a>,
    ) -> FelicoResult<AstNode<'a, Expr<'a>>> {
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
        self.create_node(&start_location, Expr::Call(CallExpr { callee, arguments }))
    }

    fn create_node<T: AstData + 'a>(
        &mut self,
        start_location: &SourceSpan<'a>,
        data: T,
    ) -> FelicoResult<AstNode<'a, T>> {
        let start = start_location;
        let end = &self.current_token.location;
        let mut location = start.clone();
        if start.start_byte != end.end_byte {
            location.end_byte = end.end_byte;
        }
        Ok(AstNode::new(data, location, self.type_factory.unknown()))
    }

    fn current_location(&self) -> SourceSpan<'a> {
        self.current_token.location.clone()
    }

    #[inline]
    fn is_at(&self, token_type: TokenType) -> bool {
        self.current_token.token_type == token_type
    }
}

pub fn parse_expression<'a>(
    code_source: SourceFile<'a>,
    workspace: &'a Workspace,
) -> FelicoResult<AstNode<'a, Expr<'a>>> {
    let parser = Parser::new(code_source, TypeFactory::new(workspace))?;
    parser.parse_expression()
}

pub fn parse_script<'a>(
    code_source: SourceFile<'a>,
    type_factory: TypeFactory<'a>,
) -> FelicoResult<AstNode<'a, Module<'a>>> {
    let mut parser = Parser::new(code_source, type_factory)?;
    parser.parse_script()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::print_ast::{ast_to_string, AstPrinter};
    use crate::infra::result::unwrap_error_result_to_string;
    use expect_test::{expect, Expect};

    fn test_parse_expression(name: &str, input: &str, expected: Expect) {
        let workspace = Workspace::new();
        let type_factory = TypeFactory::new(&workspace);
        let source_file = workspace.source_file_from_string(name, input);
        let parser = Parser::new(source_file, type_factory).unwrap();
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
            a =      [0+3]
            └── F64(2.0)     [2+1]
        "#]];
        expression_assign_twice: "a=b=3" => expect![[r#"
            a =      [0+5]
            └── b =      [2+3]
                └── F64(3.0)     [4+1]
        "#]];
        expression_assign_twice2: "a=b=3" => expect![[r#"
            a =      [0+5]
            └── b =      [2+3]
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
            Call     [0+5]
            └── Read 'foo'     [0+3]
        "#]];
        expression_call_one_arg: "foo(bar)" => expect![[r#"
            Call     [0+8]
            ├── Read 'foo'     [0+3]
            └── Read 'bar'     [4+3]
        "#]];
        expression_call_two_args: "foo(bar,baz)" => expect![[r#"
            Call     [0+12]
            ├── Read 'foo'     [0+3]
            ├── Read 'bar'     [4+3]
            └── Read 'baz'     [8+3]
        "#]];
        expression_call_with_trailing_comma: "foo(bar,baz,)" => expect![[r#"
            Call     [0+13]
            ├── Read 'foo'     [0+3]
            ├── Read 'bar'     [4+3]
            └── Read 'baz'     [8+3]
        "#]];
        expression_call_twice: "foo()()" => expect![[r#"
            Call     [0+7]
            └── Call     [0+6]
                └── Read 'foo'     [0+3]
        "#]];

    );

    fn test_parse_script(name: &str, input: &str, expected: Expect) {
        let workspace = Workspace::new();
        let type_factory = TypeFactory::new(&workspace);
        let source_file = workspace.source_file_from_string(name, input);
        let mut parser = Parser::new(source_file, type_factory).unwrap();
        let script = parser.parse_script().unwrap();
        let printed_ast = ast_to_string(&script).unwrap();

        expected.assert_eq(&printed_ast);
    }

    macro_rules! test_script {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_parse_script(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_script!(
                script_empty: "" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+0]
                        ├── Return type: Read 'unit'     [0+0]
                        └── Unit     [0+0]
                "#]];
                script_bool_true: "true;" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+5]
                        ├── Return type: Read 'unit'     [0+5]
                        ├── Bool(true)     [0+4]
                        └── Unit     [0+5]
                "#]];
                script_addition: "1+2;" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+4]
                        ├── Return type: Read 'unit'     [0+4]
                        ├── +     [0+4]
                        │   ├── F64(1.0)     [0+1]
                        │   └── F64(2.0)     [2+1]
                        └── Unit     [0+4]
                "#]];
                script_multiline: "\"Hello\";\n\"World\";" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+17]
                        ├── Return type: Read 'unit'     [0+17]
                        ├── Str("Hello")     [0+7]
                        ├── Str("World")     [9+7]
                        └── Unit     [0+17]
                "#]];

                script_true: "true;" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+5]
                        ├── Return type: Read 'unit'     [0+5]
                        ├── Bool(true)     [0+4]
                        └── Unit     [0+5]
                "#]];
                script_string_addition: "\"Hello \" + 3;" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+13]
                        ├── Return type: Read 'unit'     [0+13]
                        ├── +     [0+13]
                        │   ├── Str("Hello ")     [0+8]
                        │   └── F64(3.0)     [11+1]
                        └── Unit     [0+13]
                "#]];
                script_let_decl: "let a = false;" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+14]
                        ├── Return type: Read 'unit'     [0+14]
                        ├── Let ''a' (Identifier)'     [0+14]
                        │   └── Bool(false)     [8+5]
                        └── Unit     [0+14]
                "#]];
                script_let_decl_with_type: "let a: bool = false;" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+20]
                        ├── Return type: Read 'unit'     [0+20]
                        ├── Let ''a' (Identifier)'     [0+20]
                        │   └── Bool(false)     [14+5]
                        └── Unit     [0+20]
                "#]];
                script_script: "let a = 1;let b = a+a;b;" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+24]
                        ├── Return type: Read 'unit'     [0+24]
                        ├── Let ''a' (Identifier)'     [0+10]
                        │   └── F64(1.0)     [8+1]
                        ├── Let ''b' (Identifier)'     [10+12]
                        │   └── +     [18+4]
                        │       ├── Read 'a'     [18+1]
                        │       └── Read 'a'     [20+1]
                        ├── Read 'b'     [22+1]
                        └── Unit     [0+24]
                "#]];

                script_assign: "a=1;" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+4]
                        ├── Return type: Read 'unit'     [0+4]
                        ├── a =      [0+4]
                        │   └── F64(1.0)     [2+1]
                        └── Unit     [0+4]
                "#]];
                script_assign_twice2: "a=b=3;" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+6]
                        ├── Return type: Read 'unit'     [0+6]
                        ├── a =      [0+6]
                        │   └── b =      [2+4]
                        │       └── F64(3.0)     [4+1]
                        └── Unit     [0+6]
                "#]];
                script_block_empty: "{}" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+2]
                        ├── Return type: Read 'unit'     [0+2]
                        ├── Block     [0+2]
                        │   └── Unit     [1+1]
                        └── Unit     [0+2]
                "#]];
                script_nested_block: "{{foo;}}" => expect![[r#"
                    Module
                    └── Declare fun 'main()'     [0+8]
                        ├── Return type: Read 'unit'     [0+8]
                        ├── Block     [0+8]
                        │   ├── Block     [1+7]
                        │   │   ├── Read 'foo'     [2+3]
                        │   │   └── Unit     [6+1]
                        │   └── Unit     [7+1]
                        └── Unit     [0+8]
                "#]];

               script_if: "if(c) a;" => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+8]
                       ├── Return type: Read 'unit'     [0+8]
                       ├── If     [0+8]
                       │   ├── Read 'c'     [3+1]
                       │   └── Read 'a'     [6+1]
                       └── Unit     [0+8]
               "#]];
               script_if_else: "if(c) a else b;" => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+15]
                       ├── Return type: Read 'unit'     [0+15]
                       ├── If     [0+15]
                       │   ├── Read 'c'     [3+1]
                       │   ├── Read 'a'     [6+1]
                       │   └── Read 'b'     [13+1]
                       └── Unit     [0+15]
               "#]];
               script_if_no_semicolon: "if (c) {}" => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+9]
                       ├── Return type: Read 'unit'     [0+9]
                       ├── If     [0+9]
                       │   ├── Read 'c'     [4+1]
                       │   └── Block     [7+2]
                       │       └── Unit     [8+1]
                       └── Unit     [0+9]
               "#]];

               script_while: "while(a) b;" => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+11]
                       ├── Return type: Read 'unit'     [0+11]
                       ├── While     [0+11]
                       │   ├── Read 'a'     [6+1]
                       │   └── Read 'b'     [9+1]
                       └── Unit     [0+11]
               "#]];

               script_fun_empty: "fun foo() {}" => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+12]
                       ├── Return type: Read 'unit'     [0+12]
                       ├── Declare fun 'foo()'     [0+12]
                       │   ├── Return type: Read 'unit'     [0+11]
                       │   └── Unit     [11+1]
                       └── Unit     [0+12]
               "#]];
               script_fun_simple: "fun foo(a: bool) {a;} " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+22]
                       ├── Return type: Read 'unit'     [0+22]
                       ├── Declare fun 'foo(a)'     [0+22]
                       │   ├── Param a
                       │   │   └── Read 'bool'     [11+4]
                       │   ├── Return type: Read 'unit'     [0+18]
                       │   ├── Read 'a'     [18+1]
                       │   └── Unit     [20+1]
                       └── Unit     [0+22]
               "#]];
               script_fun_with_return_type: "fun not(a: bool,) -> bool {return !a;} " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+39]
                       ├── Return type: Read 'unit'     [0+39]
                       ├── Declare fun 'not(a)'     [0+39]
                       │   ├── Param a
                       │   │   └── Read 'bool'     [11+4]
                       │   ├── Return type: Read 'bool'     [21+4]
                       │   ├── Return     [27+10]
                       │   │   └── !     [34+3]
                       │   │       └── Read 'a'     [35+1]
                       │   └── Unit     [37+1]
                       └── Unit     [0+39]
               "#]];
               script_fun_return: "fun nop() {return;} " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+20]
                       ├── Return type: Read 'unit'     [0+20]
                       ├── Declare fun 'nop()'     [0+20]
                       │   ├── Return type: Read 'unit'     [0+11]
                       │   ├── Return     [11+7]
                       │   │   └── Unit     [11+7]
                       │   └── Unit     [18+1]
                       └── Unit     [0+20]
               "#]];
               script_fun_return_value: "fun three(a: bool) {return 3;} " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+31]
                       ├── Return type: Read 'unit'     [0+31]
                       ├── Declare fun 'three(a)'     [0+31]
                       │   ├── Param a
                       │   │   └── Read 'bool'     [13+4]
                       │   ├── Return type: Read 'unit'     [0+20]
                       │   ├── Return     [20+9]
                       │   │   └── F64(3.0)     [27+1]
                       │   └── Unit     [29+1]
                       └── Unit     [0+31]
               "#]];
               script_fun_return_expression: "fun twice(a: f64) {return a+a;} " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+32]
                       ├── Return type: Read 'unit'     [0+32]
                       ├── Declare fun 'twice(a)'     [0+32]
                       │   ├── Param a
                       │   │   └── Read 'f64'     [13+3]
                       │   ├── Return type: Read 'unit'     [0+19]
                       │   ├── Return     [19+11]
                       │   │   └── +     [26+4]
                       │   │       ├── Read 'a'     [26+1]
                       │   │       └── Read 'a'     [28+1]
                       │   └── Unit     [30+1]
                       └── Unit     [0+32]
               "#]];
               script_property_access: "a.b;" => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+4]
                       ├── Return type: Read 'unit'     [0+4]
                       ├── Get b     [0+4]
                       │   └── Read 'a'     [0+1]
                       └── Unit     [0+4]
               "#]];
               script_property_set: "a.b = 3;" => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [0+8]
                       ├── Return type: Read 'unit'     [0+8]
                       ├── Set b     [0+8]
                       │   ├── Read 'a'     [0+1]
                       │   └── F64(3.0)     [6+1]
                       └── Unit     [0+8]
               "#]];
                script_trait_simple: "
                trait Panic {
                }
               " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [17+47]
                       ├── Return type: Read 'unit'     [17+47]
                       ├── Trait 'Panic'
                       └── Unit     [17+47]
               "#]];
               script_struct_simple: "
               struct Foo {
                    bar: bool,
                    baz: f64
                }
               " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [16+106]
                       ├── Return type: Read 'unit'     [16+106]
                       ├── Struct 'Foo'     [16+106]
                       │   ├── Field bar     [49+10]
                       │   │   └── Read 'bool'     [54+4]
                       │   └── Field baz     [80+26]
                       │       └── Read 'f64'     [85+3]
                       └── Unit     [16+106]
               "#]];
               script_struct_trailing_comma: "
               struct Foo {
                    bar: bool,
                    baz: f64,
                }
               " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [16+107]
                       ├── Return type: Read 'unit'     [16+107]
                       ├── Struct 'Foo'     [16+107]
                       │   ├── Field bar     [49+10]
                       │   │   └── Read 'bool'     [54+4]
                       │   └── Field baz     [80+9]
                       │       └── Read 'f64'     [85+3]
                       └── Unit     [16+107]
               "#]];
                script_struct_empty: "
               struct Empty {}
               " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [16+31]
                       ├── Return type: Read 'unit'     [16+31]
                       ├── Struct 'Empty'     [16+31]
                       └── Unit     [16+31]
               "#]];
                script_struct_create: "
               struct Something {
                    bar: bool,
               }
                let a = Something {
                    bar: true,
                };
               " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [16+168]
                       ├── Return type: Read 'unit'     [16+168]
                       ├── Struct 'Something'     [16+86]
                       │   └── Field bar     [55+10]
                       │       └── Read 'bool'     [60+4]
                       ├── Let ''a' (Identifier)'     [99+69]
                       │   └── Create struct     [107+61]
                       │       ├── Read 'Something'     [107+9]
                       │       └── bar: Bool(true)     [144+4]
                       └── Unit     [16+168]
               "#]];
                script_struct_field_access: "
               struct Something {
                    bar: bool,
               }
                let a = Something {
                    bar: true,
                };
                a.bar = false;
                debug_print(a.bar);
               " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [16+235]
                       ├── Return type: Read 'unit'     [16+235]
                       ├── Struct 'Something'     [16+86]
                       │   └── Field bar     [55+10]
                       │       └── Read 'bool'     [60+4]
                       ├── Let ''a' (Identifier)'     [99+69]
                       │   └── Create struct     [107+61]
                       │       ├── Read 'Something'     [107+9]
                       │       └── bar: Bool(true)     [144+4]
                       ├── Set bar     [185+14]
                       │   ├── Read 'a'     [185+1]
                       │   └── Bool(false)     [193+5]
                       ├── Call     [216+19]
                       │   ├── Read 'debug_print'     [216+11]
                       │   └── Get bar     [228+6]
                       │       └── Read 'a'     [228+1]
                       └── Unit     [16+235]
               "#]];
                impl_simple: "
               struct Something {
                    bar: bool,
               }
                impl Something {
                    fun foo() {
                    }
                };
               " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [16+188]
                       ├── Return type: Read 'unit'     [16+188]
                       ├── Struct 'Something'     [16+87]
                       │   └── Field bar     [55+10]
                       │       └── Read 'bool'     [60+4]
                       ├── Impl 'Something'     [99+89]
                       │   └── Method: Declare fun 'foo()'
                       │       ├── Return type: Read 'unit'     [136+11]
                       │       └── Unit     [168+1]
                       └── Unit     [16+188]
               "#]];
                script_fib: "
                    fun fib(n: f64) {
                         return if (n <= 1) n else
                         fib(n - 2) + fib(n - 1);
                    }
                    debug_print(fib(6));
               " => expect![[r#"
                   Module
                   └── Declare fun 'main()'     [21+197]
                       ├── Return type: Read 'unit'     [21+197]
                       ├── Declare fun 'fib(n)'     [21+172]
                       │   ├── Param n
                       │   │   └── Read 'f64'     [32+3]
                       │   ├── Return type: Read 'unit'     [21+17]
                       │   ├── Return     [64+75]
                       │   │   └── If     [71+68]
                       │   │       ├── <=     [75+7]
                       │   │       │   ├── Read 'n'     [75+1]
                       │   │       │   └── F64(1.0)     [80+1]
                       │   │       ├── Read 'n'     [83+1]
                       │   │       └── +     [115+24]
                       │   │           ├── Call     [115+12]
                       │   │           │   ├── Read 'fib'     [115+3]
                       │   │           │   └── -     [119+6]
                       │   │           │       ├── Read 'n'     [119+1]
                       │   │           │       └── F64(2.0)     [123+1]
                       │   │           └── Call     [128+11]
                       │   │               ├── Read 'fib'     [128+3]
                       │   │               └── -     [132+6]
                       │   │                   ├── Read 'n'     [132+1]
                       │   │                   └── F64(1.0)     [136+1]
                       │   └── Unit     [160+1]
                       ├── Call     [182+20]
                       │   ├── Read 'debug_print'     [182+11]
                       │   └── Call     [194+7]
                       │       ├── Read 'fib'     [194+3]
                       │       └── F64(6.0)     [198+1]
                       └── Unit     [21+197]
               "#]];

    );

    fn test_parse_script_error(name: &str, input: &str, expected: Expect) {
        let workspace = Workspace::new();
        let type_factory = TypeFactory::new(&workspace);
        let source_file = workspace.source_file_from_string(name, input);
        let mut parser = Parser::new(source_file, type_factory).unwrap();
        let result = parser.parse_script();
        let diagnostic_string = unwrap_error_result_to_string(&result);
        expected.assert_eq(&diagnostic_string);
    }

    macro_rules! test_parse_error {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_parse_script_error(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_parse_error!(
        incomplete_if_statement: "if (x)" => expect![[r#"
            × Unexpected token 'EOF' in expression
               ╭─[incomplete_if_statement:1:7]
             1 │ if (x)
               ╰────

        "#]];
        unclosed_parens: "(3 4" => expect![[r#"
            × Expect closing ')' after expression, found '4' (Number) instead
               ╭─[unclosed_parens:1:4]
             1 │ (3 4
               ·    ─
               ╰────

        "#]];
         invalid_assignment: "3 = true" => expect![[r#"
             × Literal(F64(3.0)) is not a valid assignment target
                ╭─[invalid_assignment:1:1]
              1 │ 3 = true
                · ─
                ╰────
               help: Assignment target must be an l-value (e.g. a variable or field)

         "#]];
         unexpected_expression_part: "3 + if" => expect![[r#"
             × Expected '(' after 'if', found End of file instead
                ╭─[unexpected_expression_part:1:7]
              1 │ 3 + if
                ╰────

         "#]];
         incomplete_statement: "print true" => expect![[r#"
             × Expected statement terminator (';'), found 'true' (True) instead
                ╭─[incomplete_statement:1:7]
              1 │ print true
                ·       ────
                ╰────

         "#]];
         chained_values: "true \"foo\"" => expect![[r#"
             × Expected statement terminator (';'), found '"foo"' (String) instead
                ╭─[chained_values:1:6]
              1 │ true "foo"
                ·      ─────
                ╰────

         "#]];
           script_if_no_parentheses: "if c a;" => expect![[r#"
               × Expected '(' after 'if', found 'c' (Identifier) instead
                  ╭─[script_if_no_parentheses:1:4]
                1 │ if c a;
                  ·    ─
                  ╰────

           "#]];
           script_if_else_no_parentheses: "if c a else b;" => expect![[r#"
               × Expected '(' after 'if', found 'c' (Identifier) instead
                  ╭─[script_if_else_no_parentheses:1:4]
                1 │ if c a else b;
                  ·    ─
                  ╰────

           "#]];
    );
}
