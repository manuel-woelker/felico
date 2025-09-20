use crate::source_snippet::SourceSnippet;
use crate::{FilePath, SourceType};
use std::fmt::{Debug, Formatter};

pub struct SourceFile {
    path: FilePath,
    content: SourceType,
}

impl SourceFile {
    pub fn new(path: FilePath, content: SourceType) -> Self {
        Self { path, content }
    }

    pub fn in_memory(path: impl Into<FilePath>, content: impl Into<SourceType>) -> Self {
        Self::new(path.into(), content.into())
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn excerpt(&self, start: usize, end: usize) -> SourceSnippet {
        let start_line = self.content[..start].lines().count();
        SourceSnippet::new(
            self.path.clone(),
            self.content[start..end].to_string(),
            start_line,
            start,
        )
    }
}

impl Debug for SourceFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceFile")
            .field("path", &self.path)
            .finish()
    }
}
