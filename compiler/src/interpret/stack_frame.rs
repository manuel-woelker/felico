use crate::infra::source_span::SourceSpan;

#[derive(Debug, Clone)]
pub struct StackFrame<'a> {
    pub call_source_span: SourceSpan<'a>,
}
