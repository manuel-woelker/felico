use crate::ast_node::AstNode;
use crate::expression::ExpressionNode;
use crate::test_print::TestPrint;
use felico_base::result::FelicoResult;
use std::fmt::Write;
use std::ops::Deref;

pub enum Statement<'source> {
    Expression(ExpressionStatement<'source>),
}

impl<'source> Statement<'source> {
    pub fn expression(expression: ExpressionNode<'source>) -> Self {
        Self::Expression(ExpressionStatement { expression })
    }
}

pub type StatementNode<'source> = AstNode<'source, Statement<'source>>;

impl TestPrint for Statement<'_> {
    fn test_print(&self, write: &mut dyn Write, indent: usize) -> FelicoResult<()> {
        write!(write, "stmt ")?;
        match self {
            Statement::Expression(expression) => expression
                .expression
                .deref()
                .test_print(write, indent + 1)?,
        }
        Ok(())
    }
}

pub struct ExpressionStatement<'source> {
    pub expression: ExpressionNode<'source>,
}
