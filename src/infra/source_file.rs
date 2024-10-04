use miette::{MietteError, MietteSpanContents, SourceCode, SourceSpan, SpanContents};
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use crate::infra::result::FelicoResult;

#[derive(Debug)]
pub struct SourceFile {
    filename: String,
    source_code: String,
}

impl SourceFile {
    pub fn filename(&self) -> &str {
        &self.filename
    }
    pub fn source_code(&self) -> &str {
        &self.source_code
    }
}


#[derive(Debug, Clone)]
pub struct SourceFileHandle {
    inner: Arc<SourceFile>,
}

impl SourceFileHandle {
    pub fn from_path<T: AsRef<Path>>(path: T) -> FelicoResult<Self> {
        let filename = path.as_ref().as_os_str().to_owned().into_string().map_err(|_| format!("Failed to convert filename {:?}", path.as_ref()))?;
        let mut source_code = String::new();
        File::open(path)?.read_to_string(&mut source_code)?;
        Ok(Self::from_string(filename, source_code))
    }

    pub fn from_string<F: Into<String>, S: Into<String>>(filename: F, source_code: S) -> Self {
        Self {
            inner: Arc::new(SourceFile {
                filename: filename.into(),
                source_code: source_code.into(),
            }),
        }
    }

}

impl Deref for SourceFileHandle {
    type Target = SourceFile;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl SourceCode for SourceFileHandle {
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn SpanContents<'a> + 'a>, MietteError> {
        let inner_contents =
            self.source_code.read_span(span, context_lines_before, context_lines_after)?;
        let contents = MietteSpanContents::new_named(
            self.filename.clone(),
            inner_contents.data(),
            *inner_contents.span(),
            inner_contents.line(),
            inner_contents.column(),
            inner_contents.line_count(),
        );
        Ok(Box::new(contents))
    }
}