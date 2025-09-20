use crate::{FilePath, SourceType};

#[derive(Debug)]
pub struct SourceExcerpt {
    file_path: FilePath,
    source_excerpt: SourceType,
    start_line: usize,
    start_offset: usize,
}

impl SourceExcerpt {
    pub fn new(
        file_path: FilePath,
        source_excerpt: SourceType,
        start_line: usize,
        start_offset: usize,
    ) -> Self {
        Self {
            file_path,
            source_excerpt,
            start_line,
            start_offset,
        }
    }

    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
    }

    pub fn source_excerpt(&self) -> &str {
        self.source_excerpt.as_str()
    }

    pub fn start_line(&self) -> usize {
        self.start_line
    }

    pub fn start_offset(&self) -> usize {
        self.start_offset
    }
}
