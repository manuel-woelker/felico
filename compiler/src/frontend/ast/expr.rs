use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::qualified_name::QualifiedName;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::Token;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Expr<'ws> {
    Return(ReturnExpr<'ws>),
    Unary(UnaryExpr<'ws>),
    Binary(BinaryExpr<'ws>),
    Literal(LiteralExpr),
    Variable(VarUse<'ws>),
    Assign(AssignExpr<'ws>),
    Call(CallExpr<'ws>),
    Get(GetExpr<'ws>),
    Set(SetExpr<'ws>),
    Block(BlockExpr<'ws>),
    If(IfExpr<'ws>),
    CreateStruct(CreateStructExpr<'ws>),
}

impl<'ws> AstData for Expr<'ws> {}

#[derive(Debug, Clone)]
pub struct VarUse<'ws> {
    pub name: AstNode<'ws, QualifiedName<'ws>>,
    pub distance: i32,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr<'ws> {
    pub operator: Token<'ws>,
    pub left: AstNode<'ws, Expr<'ws>>,
    pub right: AstNode<'ws, Expr<'ws>>,
}

#[derive(Debug, Clone)]
pub struct AssignExpr<'ws> {
    pub destination: AstNode<'ws, QualifiedName<'ws>>,
    pub value: AstNode<'ws, Expr<'ws>>,
    pub distance: i32,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr<'ws> {
    pub operator: Token<'ws>,
    pub right: AstNode<'ws, Expr<'ws>>,
}

#[derive(Debug, Clone)]
pub enum LiteralExpr {
    Str(String),
    F64(f64),
    I64(i64),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone)]
pub struct CallExpr<'ws> {
    pub callee: AstNode<'ws, Expr<'ws>>,
    pub arguments: Vec<AstNode<'ws, Expr<'ws>>>,
}

#[derive(Debug, Clone)]
pub struct GetExpr<'ws> {
    pub object: AstNode<'ws, Expr<'ws>>,
    pub name: Token<'ws>,
}

#[derive(Debug, Clone)]
pub struct SetExpr<'ws> {
    pub object: AstNode<'ws, Expr<'ws>>,
    pub name: Token<'ws>,
    pub value: AstNode<'ws, Expr<'ws>>,
}

#[derive(Debug, Clone)]
pub struct TupleExpr<'ws> {
    pub components: Vec<AstNode<'ws, Expr<'ws>>>,
}

#[derive(Debug, Clone)]
pub struct BlockExpr<'ws> {
    pub stmts: Vec<AstNode<'ws, Stmt<'ws>>>,
    pub result_expression: AstNode<'ws, Expr<'ws>>,
}

#[derive(Debug, Clone)]
pub struct IfExpr<'ws> {
    pub condition: AstNode<'ws, Expr<'ws>>,
    pub then_expr: AstNode<'ws, Expr<'ws>>,
    pub else_expr: Option<AstNode<'ws, Expr<'ws>>>,
}

#[derive(Debug, Clone)]
pub struct ReturnExpr<'ws> {
    pub expression: AstNode<'ws, Expr<'ws>>,
}

#[derive(Debug, Clone)]
pub struct CreateStructExpr<'ws> {
    pub type_expression: AstNode<'ws, Expr<'ws>>,
    pub field_initializers: Vec<CreateStructInitializer<'ws>>,
}

#[derive(Debug, Clone)]
pub struct CreateStructInitializer<'ws> {
    pub field_name: Token<'ws>,
    pub expression: AstNode<'ws, Expr<'ws>>,
}
