use crate::frontend::ast::expr::Expr;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::Token;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression(ExprStmt),
    Let(LetStmt),
    Struct(StructStmt),
    Trait(TraitStmt),
    Impl(ImplStmt),
    Fun(FunStmt),
    While(WhileStmt),
}

impl AstData for Stmt {}

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
pub struct FunParameter {
    pub name: Token,
    pub type_expression: AstNode<Expr>,
}

impl FunParameter {
    pub fn new(name: Token, type_expression: AstNode<Expr>) -> Self {
        Self {
            name,
            type_expression,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunStmt {
    pub name: Token,
    pub parameters: Vec<FunParameter>,
    pub return_type: AstNode<Expr>,
    pub body: AstNode<Expr>,
}

impl AstData for FunStmt {}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: AstNode<Expr>,
    pub body_stmt: AstNode<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ImplStmt {
    pub name: Token,
    pub methods: Vec<AstNode<FunStmt>>,
}

#[derive(Debug, Clone)]
pub struct StructStmt {
    pub name: Token,
    pub fields: Vec<AstNode<StructStmtField>>,
}

#[derive(Debug, Clone)]
pub struct TraitStmt {
    pub name: Token,
}

#[derive(Debug, Clone)]
pub struct StructStmtField {
    pub name: Token,
    pub type_expression: AstNode<Expr>,
}

impl AstData for StructStmtField {}
