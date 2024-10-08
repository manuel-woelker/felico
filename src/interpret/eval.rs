use crate::infra::result::FelicoResult;
use crate::infra::source_file::SourceFileHandle;
use crate::interpret::interpreter::{EvalResult, Interpreter};

pub fn eval_expression(source_file: SourceFileHandle) -> FelicoResult<EvalResult> {
    Interpreter::new(source_file)?.evaluate_expression()
}

pub fn eval_program(source_file: SourceFileHandle) -> FelicoResult<()> {
    Interpreter::new(source_file)?.evaluate_program()
}
