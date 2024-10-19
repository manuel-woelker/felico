use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::AstData;
use crate::infra::full_name::FullName;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Module<'ws> {
    pub name: FullName<'ws>,
    pub stmts: Vec<AstNode<'ws, Stmt<'ws>>>,
}

impl<'ws> AstData for Module<'ws> {}
