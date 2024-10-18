use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::AstData;
use crate::infra::full_name::FullName;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Module<'a> {
    pub name: FullName,
    pub stmts: Vec<AstNode<'a, Stmt<'a>>>,
}

impl<'a> AstData for Module<'a> {}
