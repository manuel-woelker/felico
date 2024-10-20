use crate::infra::source_span::SourceSpan;

#[derive(Debug, Clone)]
pub struct StackFrame<'ws> {
    pub call_source_span: SourceSpan<'ws>,
}
