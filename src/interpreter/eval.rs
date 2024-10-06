use crate::infra::result::FelicoResult;
use crate::infra::source_file::SourceFileHandle;
use crate::interpreter::interpreter::Interpreter;
use crate::interpreter::value::InterpreterValue;

pub fn eval_expression(source_file: SourceFileHandle) -> FelicoResult<InterpreterValue> {
    Interpreter::new(source_file)?.evaluate_expression()
}

pub fn eval_program(source_file: SourceFileHandle) -> FelicoResult<()> {
    Interpreter::new(source_file)?.evaluate_program()
}
