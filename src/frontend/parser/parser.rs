use crate::frontend::ast::expr::{AssignExpr, BinaryExpr, CallExpr, Expr, GetExpr, LiteralExpr, SetExpr, UnaryExpr, VarUse};
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::program::Program;
use crate::frontend::ast::stmt::{BlockStmt, ExprStmt, FunStmt, IfStmt, ReturnStmt, Stmt, VarStmt, WhileStmt};
use crate::frontend::ast::AstData;
use crate::frontend::lexer::lexer::Lexer;
use crate::frontend::lexer::token::{Token, TokenType};
use crate::infra::diagnostic::InterpreterDiagnostic;
use crate::infra::result::{bail, failed, FelicoResult, FelicoResultExt};
use crate::infra::location::Location;
use crate::infra::source_file::SourceFileHandle;

#[derive(Debug)]
pub struct Parser {
    lexer: Lexer,
    current_token: Token,
    next_token: Token,
    source_file: SourceFileHandle,
}

impl Parser {
    pub fn new(source_file: SourceFileHandle) -> FelicoResult<Self> {
        let mut lexer = Lexer::new(source_file.clone()).whatever_context("oops")?;
        let current_token = lexer.next().ok_or_else(|| failed("Expected at least one token"))?;
        let next_token = lexer.next().unwrap_or(current_token.clone());
        Ok(Parser {
            lexer,
            current_token,
            next_token,
            source_file,
        })
    }

    pub fn new_in_memory(filename: &str, source_code: &str) -> FelicoResult<Self> {
        Self::new(SourceFileHandle::from_string(filename, source_code))
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
        self.create_node(start_location, Program {
            stmts,
        })
    }


    fn parse_decl(&mut self) -> FelicoResult<AstNode<Stmt>> {
        match self.current_token.token_type {
            TokenType::Var => {
                let node = self.parse_var_stmt()?;
                self.consume(TokenType::Semicolon, "Expected statement terminator (';')")?;
                Ok(node)
            }
            TokenType::Fun => {
                let node = self.parse_fun_stmt("function")?;
                Ok(node)
            }
            _ => {
                self.parse_stmt()
            }
        }
    }

    fn parse_var_stmt(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::Var, "var expected")?;
        let name = self.current_token.clone();
        self.consume(TokenType::Identifier, "Expected identifier after var")?;
        self.consume(TokenType::Equal, "Expected '=' in var declaration")?;
        let expression = self.parse_expr()?;
        self.create_node(start_location, Stmt::Var(VarStmt {
            name,
            expression,
        }))
    }

    fn parse_fun_stmt(&mut self, _kind: &str) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::Fun, "fun expected")?;
        let name = self.consume(TokenType::Identifier, "Expected function identifier")?;
        self.consume(TokenType::LeftParen, "Expected '('")?;
        let mut parameters = vec![];
        if self.current_token.token_type != TokenType::RightParen {
            loop {
                if parameters.len() > 255 {
                    bail!("Too many parameters");
                }
                parameters.push(self.consume(TokenType::Identifier, "Expected parameter identifier")?);
                if self.current_token.token_type != TokenType::Comma {
                    break;
                }

            }
        }
        self.consume(TokenType::RightParen, "Expected ')' to close parameter list")?;

        let body = self.parse_block()?;
        self.create_node(start_location, Stmt::Fun(FunStmt {
            name,
            parameters,
            body,
        }))
    }

    fn parse_stmt(&mut self) -> FelicoResult<AstNode<Stmt>> {
        match self.current_token.token_type {
            TokenType::LeftBrace => {
                self.parse_block()
            }
            TokenType::If => {
                self.parse_if()
            }
            TokenType::While => {
                self.parse_while()
            }
            TokenType::For => {
                self.parse_for()
            }
            TokenType::Return => {
                self.parse_return()
            }
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
            self.create_node(start_location.clone(), Expr::Literal(LiteralExpr::Nil))?
        };
        self.consume(TokenType::Semicolon, "Expected semicolon after return statement")?;
        self.create_node(start_location, Stmt::Return(ReturnStmt {
            expression,
        }))
    }
    fn parse_expr_stmt(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        let expression = self.parse_expr()?;
        self.create_node(start_location, Stmt::Expression(ExprStmt {
            expression,
        }))
    }

    fn parse_if(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::If, "Expected 'if'")?;
        self.consume(TokenType::LeftParen, "Expected '(' after if")?;
        let condition = self.parse_expr()?;
        self.consume(TokenType::RightParen, "Expected ')' after if condition")?;
        let then_stmt = self.parse_stmt()?;
        let else_stmt = if self.current_token.token_type == TokenType::Else {
            self.advance();
            Some(self.parse_stmt()?)
        } else {
            None
        };
        self.create_node(start_location, Stmt::If(IfStmt {condition, then_stmt, else_stmt}))
    }

    fn parse_while(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::While, "Expected 'while'")?;
        self.consume(TokenType::LeftParen, "Expected '(' after while")?;
        let condition = self.parse_expr()?;
        self.consume(TokenType::RightParen, "Expected ')' after while condition")?;
        let body_stmt = self.parse_stmt()?;
        self.create_node(start_location, Stmt::While(WhileStmt {condition, body_stmt}))
    }

    fn parse_for(&mut self) -> FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::For, "Expected 'for'")?;
        self.consume(TokenType::LeftParen, "Expected '(' after for")?;
        let initializer = match self.current_token.token_type {
            TokenType::Var => {
                Some(self.parse_var_stmt()?)
            }
            TokenType::Semicolon => {
                None
            }
            _ => {
                Some(self.parse_expr_stmt()?)
            }
        };
        self.consume(TokenType::Semicolon, "Expected ';' in for statement")?;
        let condition_location = self.current_location();
        let condition = match self.current_token.token_type {
            TokenType::Semicolon => {
                self.create_node(condition_location, Expr::Literal(LiteralExpr::Bool(true)))?
            }
            _ => {
                self.parse_expr()?
            }
        };
        self.consume(TokenType::Semicolon, "Expected ';' in for statement")?;
        let increment = match self.current_token.token_type {
            TokenType::RightParen => {
                None
            }
            _ => {
                Some((self.current_location(),
                self.parse_expr()?))
            }
        };
        self.consume(TokenType::RightParen, "Expected ')' in for statement")?;
        let mut body_stmt = self.parse_stmt()?;
        if let Some((start, expression)) = increment {
            let increment_stmt = self.create_node(start, Stmt::Expression(ExprStmt { expression }))?;
            body_stmt = self.create_node(body_stmt.location.clone(), Stmt::Block(BlockStmt { stmts: vec![body_stmt, increment_stmt] }))?
        }
        let mut while_stmt = self.create_node(start_location.clone(), Stmt::While(WhileStmt { condition, body_stmt }))?;
        if let Some(initializer) = initializer {
            while_stmt = self.create_node(start_location, Stmt::Block(BlockStmt { stmts: vec![initializer, while_stmt] }))?
        }
        Ok(while_stmt)
    }

    pub fn parse_block(&mut self) ->  FelicoResult<AstNode<Stmt>> {
        let start_location = self.current_location();
        self.consume(TokenType::LeftBrace, "Expected left brace ('{')")?;
        let mut stmts: Vec<AstNode<Stmt>> = vec![];

        while self.current_token.token_type != TokenType::RightBrace {
            stmts.push(self.parse_decl()?)
        }
        self.consume(TokenType::RightBrace, "Expected right brace ('}')")?;
        self.create_node(start_location, Stmt::Block(BlockStmt {
            stmts,
        }))
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

        if self.current_token.token_type == TokenType::Equal {
            self.advance();
            let value = self.parse_assignment()?;
            if let Expr::Variable(var_use) = *expr.data {
                return self.create_node(start_location, Expr::Assign(AssignExpr { destination: var_use.variable, value, distance: -2000 }));
            } else if let Expr::Get(get) = *expr.data {
                return self.create_node(start_location, Expr::Set(SetExpr { value, object: get.object, name: get.name }));
            } else {
                return self.create_diagnostic("Invalid assignment target", |diagnostic| {
                    diagnostic.add_primary_label(&expr.location);
                    diagnostic.set_help("Assignment target must be an l-value (e.g. a variable or field)");
                });
            }
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_and()?;
        while self.current_token.token_type == TokenType::Or {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_and()?;
            expr = self.create_node(start_location.clone(), Expr::Binary(BinaryExpr { operator, left: expr, right }))?
        }
        Ok(expr)
    }


    fn parse_and(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_equality()?;
        while self.current_token.token_type == TokenType::And {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_equality()?;
            expr = self.create_node(start_location.clone(), Expr::Binary(BinaryExpr { operator, left: expr, right }))?
        }
        Ok(expr)
    }
    fn parse_equality(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_comparison()?;
        while self.current_token.token_type == TokenType::BangEqual || self.current_token.token_type == TokenType::EqualEqual {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_comparison()?;
            expr = self.create_node(start_location.clone(), Expr::Binary(BinaryExpr { operator, left: expr, right }))?
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_term()?;
        while self.current_token.token_type == TokenType::Less || self.current_token.token_type == TokenType::LessEqual || self.current_token.token_type == TokenType::Greater || self.current_token.token_type == TokenType::GreaterEqual {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_term()?;
            expr = self.create_node(start_location.clone(), Expr::Binary(BinaryExpr { operator, left: expr, right }))?
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_factor()?;
        while self.current_token.token_type == TokenType::Plus || self.current_token.token_type == TokenType::Minus {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_factor()?;
            expr = self.create_node(start_location.clone(), Expr::Binary(BinaryExpr { operator, left: expr, right }))?
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> FelicoResult<AstNode<Expr>> {
        let start_location = self.current_location();
        let mut expr = self.parse_unary()?;
        while self.current_token.token_type == TokenType::Star || self.current_token.token_type == TokenType::Slash {
            let operator = self.current_token.clone();
            self.advance();
            let right = self.parse_unary()?;
            expr = self.create_node(start_location.clone(), Expr::Binary(BinaryExpr { operator, left: expr, right }))?
        }
        Ok(expr)
    }


    fn parse_primary(&mut self) -> FelicoResult<AstNode<Expr>> {
        let result = self.create_node(self.current_location(), Expr::Literal(match self.current_token.token_type {
            TokenType::Number => {
                let number: f64 = self.current_token.lexeme().parse().map_err(|e| format!("Failed to parse number '{}': {}", self.current_token.lexeme(), e))?;
                LiteralExpr::Number(number)
            }
            TokenType::String => {
                let lexeme = self.current_token.lexeme();
                let string = lexeme[1..lexeme.len() - 1].to_string();
                LiteralExpr::String(string)
            }
            TokenType::True => {
                LiteralExpr::Bool(true)
            }
            TokenType::False => {
                LiteralExpr::Bool(false)
            }
            TokenType::Identifier => {
                let result = self.create_node(self.current_location(), Expr::Variable(VarUse {variable: self.current_token.clone(), distance: -1000}));
                self.advance();
                return result;
            }
            TokenType::LeftParen => {
                self.advance();
                let result = self.parse_expr();
                self.consume(TokenType::RightParen, "Expect closing ')' after expression")?;
                return result;
            }
            _ => {
                return self.create_diagnostic(format!("Unexpected token '{}' in expression", self.current_token.token_type), |diagnostic| {
                    diagnostic.add_primary_label(&self.current_token.location);
                });
            }
        }));
        self.advance();
        result
    }

    fn consume(&mut self, expected_token_type: TokenType, msg: &str) -> FelicoResult<Token> {
        if self.current_token.token_type == expected_token_type {
            let token = self.current_token.clone();
            self.advance();
            Ok(token)
        } else {
            self.create_diagnostic(format!("{}, found {} instead", msg, self.current_token), |diagnostic| {
                diagnostic.add_primary_label(&self.current_token.location)
            })
        }
    }

    fn create_diagnostic<T, S: Into<String>>(&self, message: S, mut f: impl FnMut(&mut InterpreterDiagnostic) -> ()) -> FelicoResult<T> {
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
            _ => {
                self.parse_call()
            }
        }
    }


    fn parse_call(&mut self) -> FelicoResult<AstNode<Expr>> {
        let mut expr = self.parse_primary()?;
        let start_location = self.current_location();
        loop {
            if self.current_token.token_type == TokenType::LeftParen {
                self.advance();
                expr = self.finish_call(expr, start_location.clone())?;
            } else if self.current_token.token_type == TokenType::Dot {
                self.advance();
                let name = self.consume(TokenType::Identifier, "Expected identifier after '.'")?;
                expr = self.create_node(start_location.clone(), Expr::Get(GetExpr {object: expr, name}))?;
            } else{
                break;
            }
        };
        Ok(expr)
    }

    fn finish_call(&mut self, callee: AstNode<Expr>, start_location: Location) -> FelicoResult<AstNode<Expr>> {
        let mut arguments: Vec<AstNode<Expr>> = vec![];
        if self.current_token.token_type != TokenType::RightParen {
            loop {
                if arguments.len() >= 255 {
                    bail!("Too many arguments in call expression");
                }
                arguments.push(self.parse_expr()?);
                if self.current_token.token_type == TokenType::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expected ')' after function call arguments")?;
        self.create_node(start_location, Expr::Call(CallExpr {callee, arguments}))
    }

    fn create_node<T: AstData>(&mut self, start_location: Location, data: T) -> FelicoResult<AstNode<T>> {
        let start = start_location;
        let end = &self.current_token.location;
        let mut location = start.clone();
        if start.start_byte != end.end_byte {
            location.end_byte = end.end_byte;
        }
        Ok(AstNode::new(data, location))
    }

    fn current_location(&self) -> Location {
        self.current_token.location.clone()
    }

}

pub fn parse_expression(code_source: SourceFileHandle) -> FelicoResult<AstNode<Expr>> {
    let parser = Parser::new(code_source)?;
    parser.parse_expression()
}

pub fn parse_program(code_source: SourceFileHandle) -> FelicoResult<AstNode<Program>> {
    let parser = Parser::new(code_source)?;
    parser.parse_program()
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::print_ast::{print_expr_ast, print_program_ast};
    use crate::infra::diagnostic::unwrap_diagnostic_to_string;
    use expect_test::{expect, Expect};
    use std::io::Cursor;

    fn test_parse_expression(name: &str, input: &str, expected: Expect) {
        let parser = Parser::new_in_memory(name, input).unwrap();
        let expr = parser.parse_expression().unwrap();
        let mut buffer = Cursor::new(Vec::<u8>::new());
        print_expr_ast(&expr, &mut buffer).unwrap();
        let printed_ast = String::from_utf8(buffer.into_inner()).unwrap();
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
            String("")     [0+2]
        "#]];
        string_space: "\" \"" => expect![[r#"
            String(" ")     [0+3]
        "#]];
        string_newline: "\"\n\"" => expect![[r#"
            String("\n")     [0+3]
        "#]];
        string_foo: "\"foo\"" => expect![[r#"
            String("foo")     [0+5]
        "#]];
        string_unicode: "\"😶‍🌫️\"" => expect![[r#"
            String("😶\u{200d}🌫\u{fe0f}")     [0+16]
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
            Number(0.0)     [0+1]
        "#]];
        number_literal_123: "123" => expect![[r#"
            Number(123.0)     [0+3]
        "#]];
        number_literal_3_141: "3.141" => expect![[r#"
            Number(3.141)     [0+5]
        "#]];
        number_literal_minus_1: "-1.0" => expect![[r#"
            -     [0+4]
            └── Number(1.0)     [1+3]
        "#]];
        expression_equal_equal: "1.0==2.0" => expect![[r#"
            ==     [0+8]
            ├── Number(1.0)     [0+3]
            └── Number(2.0)     [5+3]
        "#]];
        bang_equal_equal: "-1 == - 10" => expect![[r#"
            ==     [0+10]
            ├── -     [0+5]
            │   └── Number(1.0)     [1+1]
            └── -     [6+4]
                └── Number(10.0)     [8+2]
        "#]];
        expression_less: "1<2" => expect![[r#"
            <     [0+3]
            ├── Number(1.0)     [0+1]
            └── Number(2.0)     [2+1]
        "#]];
        expression_less_equal: "1<=2" => expect![[r#"
            <=     [0+4]
            ├── Number(1.0)     [0+1]
            └── Number(2.0)     [3+1]
        "#]];
        expression_greater: "1>2" => expect![[r#"
            >     [0+3]
            ├── Number(1.0)     [0+1]
            └── Number(2.0)     [2+1]
        "#]];
        expression_greater_equal: "1>=2" => expect![[r#"
            >=     [0+4]
            ├── Number(1.0)     [0+1]
            └── Number(2.0)     [3+1]
        "#]];
        expression_precedence: "true == 1>=2" => expect![[r#"
            ==     [0+12]
            ├── Bool(true)     [0+4]
            └── >=     [8+4]
                ├── Number(1.0)     [8+1]
                └── Number(2.0)     [11+1]
        "#]];
        expression_plus: "1+2" => expect![[r#"
            +     [0+3]
            ├── Number(1.0)     [0+1]
            └── Number(2.0)     [2+1]
        "#]];
        expression_minus: "1-2" => expect![[r#"
            -     [0+3]
            ├── Number(1.0)     [0+1]
            └── Number(2.0)     [2+1]
        "#]];
        expression_times: "1*2" => expect![[r#"
            *     [0+3]
            ├── Number(1.0)     [0+1]
            └── Number(2.0)     [2+1]
        "#]];
        expression_division: "1/2" => expect![[r#"
            /     [0+3]
            ├── Number(1.0)     [0+1]
            └── Number(2.0)     [2+1]
        "#]];
        expression_precedence_math: "4==1+2*-3" => expect![[r#"
            ==     [0+9]
            ├── Number(4.0)     [0+1]
            └── +     [3+6]
                ├── Number(1.0)     [3+1]
                └── *     [5+4]
                    ├── Number(2.0)     [5+1]
                    └── -     [7+2]
                        └── Number(3.0)     [8+1]
        "#]];
        expression_paren_simple: "(1)" => expect![[r#"
            Number(1.0)     [1+1]
        "#]];
        expression_paren_nexted: "((1))" => expect![[r#"
            Number(1.0)     [2+1]
        "#]];
        expression_paren_complex: "3*(1+2)" => expect![[r#"
            *     [0+7]
            ├── Number(3.0)     [0+1]
            └── +     [3+4]
                ├── Number(1.0)     [3+1]
                └── Number(2.0)     [5+1]
        "#]];
        expression_assign: "a=2" => expect![[r#"
            'a' (Identifier) =      [0+3]
            └── Number(2.0)     [2+1]
        "#]];
        expression_assign_twice: "a=b=3" => expect![[r#"
            'a' (Identifier) =      [0+5]
            └── 'b' (Identifier) =      [2+3]
                └── Number(3.0)     [4+1]
        "#]];
        expression_assign_twice2: "a=b=3" => expect![[r#"
            'a' (Identifier) =      [0+5]
            └── 'b' (Identifier) =      [2+3]
                └── Number(3.0)     [4+1]
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
        expression_call_twice: "foo()()" => expect![[r#"
            Call     [3+4]
            └── Call     [3+3]
                └── Read 'foo'     [0+3]
        "#]];
    );


    fn test_parse_program(name: &str, input: &str, expected: Expect) {
        let parser = Parser::new_in_memory(name, input).unwrap();
        let program = parser.parse_program().unwrap();
        let mut buffer = Cursor::new(Vec::<u8>::new());
        print_program_ast(&program, &mut buffer).unwrap();
        let printed_ast = String::from_utf8(buffer.into_inner()).unwrap();

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
                        ├── Number(1.0)     [0+1]
                        └── Number(2.0)     [2+1]
                "#]];
                program_multiline: "\"Hello\";\n\"World\";" => expect![[r#"
                    Program
                    ├── String("Hello")     [0+7]     [0+8]
                    └── String("World")     [9+7]     [9+8]
                "#]];

                program_true: "true;" => expect![[r#"
                    Program
                    └── Bool(true)     [0+4]     [0+5]
                "#]];
                program_string_addition: "\"Hello \" + 3;" => expect![[r#"
                    Program
                    └── +     [0+13]     [0+13]
                        ├── String("Hello ")     [0+8]
                        └── Number(3.0)     [11+1]
                "#]];
                program_var_decl: "var a = false;" => expect![[r#"
                    Program
                    └── Declare var ''a' (Identifier)'     [0+14]
                        └── Bool(false)     [8+5]
                "#]];
                program_program: "var a = 1;var b = a+a;b;" => expect![[r#"
                    Program
                    ├── Declare var ''a' (Identifier)'     [0+10]
                    │   └── Number(1.0)     [8+1]
                    ├── Declare var ''b' (Identifier)'     [10+12]
                    │   └── +     [18+4]
                    │       ├── Read 'a'     [18+1]
                    │       └── Read 'a'     [20+1]
                    └── Read 'b'     [22+1]     [22+2]
                "#]];

                program_assign: "a=1;" => expect![[r#"
                    Program
                    └── 'a' (Identifier) =      [0+4]     [0+4]
                        └── Number(1.0)     [2+1]
                "#]];
                program_assign_twice2: "a=b=3;" => expect![[r#"
                    Program
                    └── 'a' (Identifier) =      [0+6]     [0+6]
                        └── 'b' (Identifier) =      [2+4]
                            └── Number(3.0)     [4+1]
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

               program_for_var: "for(var i = 1; i < 3; i = i + 1) i;" => expect![[r#"
                   Program
                   └── Block     [0+35]
                       ├── Declare var ''i' (Identifier)'     [4+10]
                       │   └── Number(1.0)     [12+1]
                       └── While     [0+35]
                           ├── <     [15+6]
                           │   ├── Read 'i'     [15+1]
                           │   └── Number(3.0)     [19+1]
                           └── Block     [33+2]
                               ├── Read 'i'     [33+1]     [33+2]
                               └── 'i' (Identifier) =      [22+10]     [22+13]
                                   └── +     [26+6]
                                       ├── Read 'i'     [26+1]
                                       └── Number(1.0)     [30+1]
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
               "#]];
               program_fun_simple: "fun foo(a) {a;} " => expect![[r#"
                   Program
                   └── Declare fun 'foo(a)'     [0+16]
                       └── Read 'a'     [12+1]     [12+2]
               "#]];
               program_fun_return: "fun nop() {return;} " => expect![[r#"
                   Program
                   └── Declare fun 'nop()'     [0+20]
                       └── Return     [11+8]
                           └── Nil     [11+7]
               "#]];
               program_fun_return_value: "fun three(a) {return 3;} " => expect![[r#"
                   Program
                   └── Declare fun 'three(a)'     [0+25]
                       └── Return     [14+10]
                           └── Number(3.0)     [21+1]
               "#]];
               program_fun_return_expression: "fun twice(a) {return a+a;} " => expect![[r#"
                   Program
                   └── Declare fun 'twice(a)'     [0+27]
                       └── Return     [14+12]
                           └── +     [21+4]
                               ├── Read 'a'     [21+1]
                               └── Read 'a'     [23+1]
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
                       └── Number(3.0)     [6+1]
               "#]];

    );

    fn test_parse_program_error(name: &str, input: &str, expected: Expect) {
        let parser = Parser::new_in_memory(name, input).unwrap();
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