use crate::infra::result::FelicoResult;
use crate::infra::source_file::SourceFile;
use crate::interpret::interpreter::Interpreter;
use crate::interpret::value::InterpreterValue;

pub fn eval_expression<'a>(source_file: SourceFile) -> FelicoResult<InterpreterValue<'a>> {
    Interpreter::new(source_file)?.evaluate_expression()
}

pub fn eval_program(source_file: SourceFile) -> FelicoResult<()> {
    Interpreter::new(source_file)?.run()
}
