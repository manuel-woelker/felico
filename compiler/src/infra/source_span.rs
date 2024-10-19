use crate::infra::source_file::{SourceFile, SourceFileInner};
use std::fmt::{Debug, Formatter};

pub type ByteOffset = i32;

#[derive(Clone)]
pub struct SourceSpan<'ws> {
    pub source_file: SourceFile<'ws>,
    pub start_byte: ByteOffset,
    pub end_byte: ByteOffset,
}
const EPHEMERAL_FILE: &str = "<ephemeral file>";
const EPHEMERAL_SOURCE_FILE_INNER: SourceFileInner = SourceFileInner {
    filename: EPHEMERAL_FILE,
    source_code: "",
};

impl<'ws> SourceSpan<'ws> {
    pub fn ephemeral() -> SourceSpan<'ws> {
        SourceSpan {
            source_file: SourceFile {
                inner: &EPHEMERAL_SOURCE_FILE_INNER,
            },
            start_byte: 0,
            end_byte: 0,
        }
    }

    pub fn is_ephemeral(&self) -> bool {
        self.source_file.filename() == EPHEMERAL_FILE
    }

    pub fn get_source_code(&self) -> &str {
        &self.source_file.source_code()[self.start_byte as usize..self.end_byte as usize]
    }
    pub fn get_line_number(&self) -> usize {
        self.source_file.source_code().as_bytes()[0..self.start_byte as usize]
            .iter()
            .filter(|&&c| c == b'\n')
            .count()
            + 1
    }
    pub fn get_column_number(&self) -> usize {
        self.source_file.source_code().as_bytes()[0..self.start_byte as usize]
            .iter()
            .rev()
            .take_while(|&&c| c != b'\n')
            .count()
            + 1
    }
}

impl<'ws> Debug for SourceSpan<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}:{}:{}]",
            self.source_file.filename(),
            self.get_line_number(),
            self.get_column_number()
        )
    }
}
