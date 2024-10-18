use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::qualified_name::QualifiedName;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::Token;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Expr<'a> {
    Return(ReturnExpr<'a>),
    Unary(UnaryExpr<'a>),
    Binary(BinaryExpr<'a>),
    Literal(LiteralExpr),
    Variable(VarUse<'a>),
    Assign(AssignExpr<'a>),
    Call(CallExpr<'a>),
    Get(GetExpr<'a>),
    Set(SetExpr<'a>),
    Block(BlockExpr<'a>),
    If(IfExpr<'a>),
    CreateStruct(CreateStructExpr<'a>),
}

impl<'a> AstData for Expr<'a> {}

#[derive(Debug, Clone)]
pub struct VarUse<'a> {
    pub name: AstNode<'a, QualifiedName>,
    pub distance: i32,
}

#[derive(Debug, Clone)]
pub struct BinaryExpr<'a> {
    pub operator: Token,
    pub left: AstNode<'a, Expr<'a>>,
    pub right: AstNode<'a, Expr<'a>>,
}

#[derive(Debug, Clone)]
pub struct AssignExpr<'a> {
    pub destination: AstNode<'a, QualifiedName>,
    pub value: AstNode<'a, Expr<'a>>,
    pub distance: i32,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr<'a> {
    pub operator: Token,
    pub right: AstNode<'a, Expr<'a>>,
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
pub struct CallExpr<'a> {
    pub callee: AstNode<'a, Expr<'a>>,
    pub arguments: Vec<AstNode<'a, Expr<'a>>>,
}

#[derive(Debug, Clone)]
pub struct GetExpr<'a> {
    pub object: AstNode<'a, Expr<'a>>,
    pub name: Token,
}

#[derive(Debug, Clone)]
pub struct SetExpr<'a> {
    pub object: AstNode<'a, Expr<'a>>,
    pub name: Token,
    pub value: AstNode<'a, Expr<'a>>,
}

#[derive(Debug, Clone)]
pub struct TupleExpr<'a> {
    pub components: Vec<AstNode<'a, Expr<'a>>>,
}

#[derive(Debug, Clone)]
pub struct BlockExpr<'a> {
    pub stmts: Vec<AstNode<'a, Stmt<'a>>>,
    pub result_expression: AstNode<'a, Expr<'a>>,
}

#[derive(Debug, Clone)]
pub struct IfExpr<'a> {
    pub condition: AstNode<'a, Expr<'a>>,
    pub then_expr: AstNode<'a, Expr<'a>>,
    pub else_expr: Option<AstNode<'a, Expr<'a>>>,
}

#[derive(Debug, Clone)]
pub struct ReturnExpr<'a> {
    pub expression: AstNode<'a, Expr<'a>>,
}

#[derive(Debug, Clone)]
pub struct CreateStructExpr<'a> {
    pub type_expression: AstNode<'a, Expr<'a>>,
    pub field_initializers: Vec<CreateStructInitializer<'a>>,
}

#[derive(Debug, Clone)]
pub struct CreateStructInitializer<'a> {
    pub field_name: Token,
    pub expression: AstNode<'a, Expr<'a>>,
}
