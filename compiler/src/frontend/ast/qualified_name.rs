use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::Token;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct QualifiedName {
    pub parts: Vec<Token>,
}

impl AstData for QualifiedName {}

impl Display for AstNode<QualifiedName> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for part in &self.data.parts {
            if !first {
                f.write_str("::")?;
            }
            first = false;
            f.write_str(part.lexeme())?
        }
        Ok(())
    }
}
