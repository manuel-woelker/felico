use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::Token;
use crate::infra::full_name::FullName;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct QualifiedName<'ws> {
    pub parts: Vec<Token<'ws>>,
    pub full_name: FullName<'ws>,
}

impl<'ws> AstData for QualifiedName<'ws> {}

impl<'ws> Display for AstNode<'ws, QualifiedName<'ws>> {
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
