use crate::source_excerpt::SourceExcerpt;
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

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn excerpt(&self, start: usize, end: usize) -> SourceExcerpt {
        let start_line = self.content[..start].lines().count();
        SourceExcerpt::new(
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
