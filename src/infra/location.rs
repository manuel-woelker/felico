use crate::infra::source_file::SourceFileHandle;

pub type ByteOffset = i32;

#[derive(Debug, Clone)]
pub struct Location {
    pub source_file: SourceFileHandle,
    pub start_byte: ByteOffset,
    pub end_byte: ByteOffset,
    pub line: ByteOffset,
    pub column: ByteOffset,
}