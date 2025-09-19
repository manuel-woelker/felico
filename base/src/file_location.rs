use crate::source_file::SourceFile;

#[derive(Debug)]
pub struct FileLocation<'source> {
    pub source_file: &'source SourceFile,
    pub start: usize,
    pub end: usize,
}

impl<'source> FileLocation<'source> {
    pub fn new(source_file: &'source SourceFile, start: usize, end: usize) -> Self {
        Self {
            source_file,
            start,
            end,
        }
    }
}
