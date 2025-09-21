use crate::ast_node::AstNode;
use crate::identifier::IdentifierNode;
use crate::statement::StatementNode;
use crate::test_print::TestPrint;
use felico_base::result::FelicoResult;
use std::fmt::Write;
use std::ops::Deref;

pub struct FunDefinition<'source> {
    pub name: IdentifierNode<'source>,
    pub statements: Vec<StatementNode<'source>>,
}

impl<'source> FunDefinition<'source> {
    pub fn new(name: IdentifierNode<'source>, statements: Vec<StatementNode<'source>>) -> Self {
        Self { name, statements }
    }
}

pub type FunDefinitionNode<'source> = AstNode<'source, FunDefinition<'source>>;

impl TestPrint for FunDefinition<'_> {
    fn test_print(&self, write: &mut dyn Write, indent: usize) -> FelicoResult<()> {
        write!(write, "fun ")?;
        self.name.deref().test_print(write, indent + 1)?;
        writeln!(write)?;
        for statement in &self.statements {
            statement.test_print(write, indent + 1)?;
        }
        Ok(())
    }
}
