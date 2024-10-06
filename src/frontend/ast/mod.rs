use std::fmt::Debug;

pub mod expr;
pub mod node;
pub mod stmt;
pub mod program;
pub mod print_ast;
pub mod types;

pub trait AstData: Debug + 'static {}
