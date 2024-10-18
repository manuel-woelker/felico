use crate::frontend::ast::expr::Expr;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::Token;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Stmt<'a> {
    Expression(ExprStmt<'a>),
    Let(LetStmt<'a>),
    Struct(StructStmt<'a>),
    Trait(TraitStmt),
    Impl(ImplStmt<'a>),
    Fun(FunStmt<'a>),
    While(WhileStmt<'a>),
}

impl<'a> AstData for Stmt<'a> {}

#[derive(Debug, Clone)]
pub struct ExprStmt<'a> {
    pub expression: AstNode<'a, Expr<'a>>,
}

#[derive(Debug, Clone)]
pub struct LetStmt<'a> {
    pub name: Token,
    pub expression: AstNode<'a, Expr<'a>>,
    pub type_expression: Option<AstNode<'a, Expr<'a>>>,
}

#[derive(Debug, Clone)]
pub struct FunParameter<'a> {
    pub name: Token,
    pub type_expression: AstNode<'a, Expr<'a>>,
}

impl<'a> FunParameter<'a> {
    pub fn new(name: Token, type_expression: AstNode<'a, Expr<'a>>) -> Self {
        Self {
            name,
            type_expression,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunStmt<'a> {
    pub name: Token,
    pub parameters: Vec<FunParameter<'a>>,
    pub return_type: AstNode<'a, Expr<'a>>,
    pub body: AstNode<'a, Expr<'a>>,
}

impl<'a> AstData for FunStmt<'a> {}

#[derive(Debug, Clone)]
pub struct WhileStmt<'a> {
    pub condition: AstNode<'a, Expr<'a>>,
    pub body_stmt: AstNode<'a, Stmt<'a>>,
}

#[derive(Debug, Clone)]
pub struct ImplStmt<'a> {
    pub name: Token,
    pub methods: Vec<AstNode<'a, FunStmt<'a>>>,
}

#[derive(Debug, Clone)]
pub struct StructStmt<'a> {
    pub name: Token,
    pub fields: Vec<AstNode<'a, StructStmtField<'a>>>,
}

#[derive(Debug, Clone)]
pub struct TraitStmt {
    pub name: Token,
}

#[derive(Debug, Clone)]
pub struct StructStmtField<'a> {
    pub name: Token,
    pub type_expression: AstNode<'a, Expr<'a>>,
}

impl<'a> AstData for StructStmtField<'a> {}
