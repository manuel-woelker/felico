use crate::ast_node::AstNode;
use crate::test_print::TestPrint;
use felico_base::result::FelicoResult;
use std::fmt::Write;

pub struct Identifier {
    pub name: String,
}

pub type IdentifierNode<'source> = AstNode<'source, Identifier>;

impl TestPrint for Identifier {
    fn test_print(&self, write: &mut dyn Write, indent: usize) -> FelicoResult<()> {
        write!(write, "{} Identifier {}", "\t".repeat(indent), self.name)?;
        Ok(())
    }
}
