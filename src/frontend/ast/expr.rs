use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::AstData;
use crate::frontend::lexer::token::Token;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Expr {
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Literal(LiteralExpr),
    Variable(VarUse),
    Assign(AssignExpr),
    Call(CallExpr),
    Get(GetExpr),
    Set(SetExpr),
}

impl AstData for Expr {}

#[derive(Debug, Clone)]
pub struct VarUse {
    pub variable: Token,
    pub distance: i32,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub operator: Token,
    pub left: AstNode<Expr>,
    pub right: AstNode<Expr>,
}

#[derive(Debug, Clone)]
pub struct AssignExpr {
    pub destination: Token,
    pub value: AstNode<Expr>,
    pub distance: i32,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub operator: Token,
    pub right: AstNode<Expr>,
}

#[derive(Debug, Clone)]
pub enum LiteralExpr {
    String(String),
    Number(f64),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: AstNode<Expr>,
    pub arguments: Vec<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct GetExpr {
    pub object: AstNode<Expr>,
    pub name: Token,
}

#[derive(Debug, Clone)]
pub struct SetExpr {
    pub object: AstNode<Expr>,
    pub name: Token,
    pub value: AstNode<Expr>,
}
