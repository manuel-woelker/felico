use crate::frontend::ast::expr::Expr;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::Token;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Stmt<'ws> {
    Expression(ExprStmt<'ws>),
    Let(LetStmt<'ws>),
    Struct(StructStmt<'ws>),
    Trait(TraitStmt<'ws>),
    Impl(ImplStmt<'ws>),
    Fun(FunStmt<'ws>),
    While(WhileStmt<'ws>),
}

impl<'ws> AstData for Stmt<'ws> {}

#[derive(Debug, Clone)]
pub struct ExprStmt<'ws> {
    pub expression: AstNode<'ws, Expr<'ws>>,
}

#[derive(Debug, Clone)]
pub struct LetStmt<'ws> {
    pub name: Token<'ws>,
    pub expression: AstNode<'ws, Expr<'ws>>,
    pub type_expression: Option<AstNode<'ws, Expr<'ws>>>,
}

#[derive(Debug, Clone)]
pub struct FunParameter<'ws> {
    pub name: Token<'ws>,
    pub type_expression: AstNode<'ws, Expr<'ws>>,
}

impl<'ws> FunParameter<'ws> {
    pub fn new(name: Token<'ws>, type_expression: AstNode<'ws, Expr<'ws>>) -> Self {
        Self {
            name,
            type_expression,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunStmt<'ws> {
    pub name: Token<'ws>,
    pub parameters: Vec<FunParameter<'ws>>,
    pub return_type: AstNode<'ws, Expr<'ws>>,
    pub body: AstNode<'ws, Expr<'ws>>,
}

impl<'ws> AstData for FunStmt<'ws> {}

#[derive(Debug, Clone)]
pub struct WhileStmt<'ws> {
    pub condition: AstNode<'ws, Expr<'ws>>,
    pub body_stmt: AstNode<'ws, Stmt<'ws>>,
}

#[derive(Debug, Clone)]
pub struct ImplStmt<'ws> {
    pub name: Token<'ws>,
    pub methods: Vec<AstNode<'ws, FunStmt<'ws>>>,
}

#[derive(Debug, Clone)]
pub struct StructStmt<'ws> {
    pub name: Token<'ws>,
    pub fields: Vec<AstNode<'ws, StructStmtField<'ws>>>,
}

#[derive(Debug, Clone)]
pub struct TraitStmt<'ws> {
    pub name: Token<'ws>,
}

#[derive(Debug, Clone)]
pub struct StructStmtField<'ws> {
    pub name: Token<'ws>,
    pub type_expression: AstNode<'ws, Expr<'ws>>,
}

impl<'ws> AstData for StructStmtField<'ws> {}
