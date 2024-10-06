use crate::frontend::ast::expr::Expr;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::Token;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Stmt {
    Return(ReturnStmt),
    Expression(ExprStmt),
    Let(LetStmt),
    Fun(FunStmt),
    Block(BlockStmt),
    If(IfStmt),
    While(WhileStmt),
}

impl AstData for Stmt {}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub expression: AstNode<Expr>,
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expression: AstNode<Expr>,
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: Token,
    pub expression: AstNode<Expr>,
    pub type_expression: Option<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct FunStmt {
    pub name: Token,
    pub parameters: Vec<Token>,
    pub body: AstNode<Stmt>,
}

impl AstData for FunStmt {}

#[derive(Debug, Clone)]
pub struct BlockStmt {
    pub stmts: Vec<AstNode<Stmt>>,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: AstNode<Expr>,
    pub then_stmt: AstNode<Stmt>,
    pub else_stmt: Option<AstNode<Stmt>>,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: AstNode<Expr>,
    pub body_stmt: AstNode<Stmt>,
}
