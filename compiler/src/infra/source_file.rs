use crate::model::workspace::WorkspaceString;
use miette::{MietteError, MietteSpanContents, SourceCode, SourceSpan, SpanContents};
use std::fmt::Debug;

#[derive(Debug)]
pub struct SourceFileInner<'a> {
    pub filename: WorkspaceString<'a>,
    pub source_code: WorkspaceString<'a>,
}

#[derive(Debug, Clone)]
pub struct SourceFile<'a> {
    pub inner: &'a SourceFileInner<'a>,
}

impl<'a> SourceFile<'a> {
    pub fn filename(&self) -> &'a str {
        &self.inner.filename
    }
    pub fn source_code(&self) -> &'a str {
        &self.inner.source_code
    }
}

impl<'a> Copy for SourceFile<'a> {}

impl<'workspace> SourceCode for SourceFile<'workspace> {
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn SpanContents<'a> + 'a>, MietteError> {
        let inner_contents =
            self.inner
                .source_code
                .read_span(span, context_lines_before, context_lines_after)?;
        let contents = MietteSpanContents::new_named(
            self.inner.filename.to_string(),
            inner_contents.data(),
            *inner_contents.span(),
            inner_contents.line(),
            inner_contents.column(),
            inner_contents.line_count(),
        );
        Ok(Box::new(contents))
    }
}
