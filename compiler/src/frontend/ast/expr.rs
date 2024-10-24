use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::qualified_name::QualifiedName;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::Token;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Expr {
    Return(ReturnExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Literal(LiteralExpr),
    Variable(VarUse),
    Assign(AssignExpr),
    Call(CallExpr),
    Get(GetExpr),
    Set(SetExpr),
    Block(BlockExpr),
    If(IfExpr),
    CreateStruct(CreateStructExpr),
}

impl AstData for Expr {}

#[derive(Debug, Clone)]
pub struct VarUse {
    pub name: AstNode<QualifiedName>,
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
    pub destination: AstNode<QualifiedName>,
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
    Str(String),
    F64(f64),
    I64(i64),
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

#[derive(Debug, Clone)]
pub struct TupleExpr {
    pub components: Vec<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct BlockExpr {
    pub stmts: Vec<AstNode<Stmt>>,
    pub result_expression: AstNode<Expr>,
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: AstNode<Expr>,
    pub then_expr: AstNode<Expr>,
    pub else_expr: Option<AstNode<Expr>>,
}

#[derive(Debug, Clone)]
pub struct ReturnExpr {
    pub expression: AstNode<Expr>,
}

#[derive(Debug, Clone)]
pub struct CreateStructExpr {
    pub type_expression: AstNode<Expr>,
    pub field_initializers: Vec<CreateStructInitializer>,
}

#[derive(Debug, Clone)]
pub struct CreateStructInitializer {
    pub field_name: Token,
    pub expression: AstNode<Expr>,
}
