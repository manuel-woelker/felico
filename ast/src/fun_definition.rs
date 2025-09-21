use crate::ast_node::AstNode;
use crate::identifier::IdentifierNode;
use crate::test_print::TestPrint;
use felico_base::result::FelicoResult;
use std::fmt::Write;

pub struct FunDefinition<'source> {
    pub name: IdentifierNode<'source>,
}

impl<'source> FunDefinition<'source> {
    pub fn new(name: IdentifierNode<'source>) -> Self {
        Self { name }
    }
}

pub type FunDefinitionNode<'source> = AstNode<'source, FunDefinition<'source>>;

impl TestPrint for FunDefinition<'_> {
    fn test_print(&self, write: &mut dyn Write, _indent: usize) -> FelicoResult<()> {
        writeln!(write, "fun {}", self.name.name())?;
        Ok(())
    }
}
