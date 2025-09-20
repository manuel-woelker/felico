use crate::ast_node::AstNode;
use crate::test_print::TestPrint;
use felico_base::result::FelicoResult;
use std::fmt::Write;

#[derive(Debug, Default)]
pub struct CompilationUnit {}

impl CompilationUnit {
    pub fn new() -> Self {
        Self {}
    }
}

pub type CompilationUnitAst<'source> = AstNode<'source, CompilationUnit>;

impl TestPrint for CompilationUnit {
    fn test_print(&self, write: &mut dyn Write, indent: usize) -> FelicoResult<()> {
        writeln!(write, "{} Compilation Unit", "\t".repeat(indent))?;
        Ok(())
    }
}
