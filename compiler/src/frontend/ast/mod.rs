use std::fmt::Debug;

pub mod expr;
pub mod module;
pub mod node;
pub mod print_ast;
pub mod qualified_name;
pub mod stmt;

pub trait AstData: Debug {}
