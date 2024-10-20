use crate::infra::result::FelicoResult;
use crate::infra::source_file::SourceFile;
use crate::interpret::interpreter::Interpreter;
use crate::interpret::value::InterpreterValue;
use crate::model::workspace::Workspace;

pub fn eval_expression<'ws>(
    workspace: Workspace<'ws>,
    source_file: SourceFile<'ws>,
) -> FelicoResult<InterpreterValue<'ws>> {
    Interpreter::new(workspace, source_file)?.evaluate_expression()
}

pub fn eval_program<'ws>(
    workspace: Workspace<'ws>,
    source_file: SourceFile<'ws>,
) -> FelicoResult<()> {
    Interpreter::new(workspace, source_file)?.run()
}
