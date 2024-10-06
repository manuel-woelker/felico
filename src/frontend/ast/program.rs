use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::AstData;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Program {
    pub stmts: Vec<AstNode<Stmt>>,
}

impl AstData for Program {}
