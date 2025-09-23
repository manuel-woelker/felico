use crate::ast_node::AstNode;
use crate::fun_definition::FunDefinitionNode;
use felico_base::result::FelicoResult;
use felico_base::test_print::TestPrint;
use std::fmt::Write;

pub struct CompilationUnit<'source> {
    pub fun_definitions: Vec<FunDefinitionNode<'source>>,
}

impl<'source> CompilationUnit<'source> {
    pub fn new(fun_definitions: Vec<FunDefinitionNode<'source>>) -> Self {
        Self { fun_definitions }
    }
}

pub type CompilationUnitNode<'source> = AstNode<'source, CompilationUnit<'source>>;

impl TestPrint for CompilationUnit<'_> {
    fn test_print(&self, write: &mut dyn Write, indent: usize) -> FelicoResult<()> {
        writeln!(write, "{} Compilation Unit", "\t".repeat(indent))?;
        for fun_definition in &self.fun_definitions {
            fun_definition.test_print(write, indent + 1)?;
        }
        Ok(())
    }
}
