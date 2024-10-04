use crate::infra::source_file::SourceFileHandle;

pub type OffsetType = i32;

#[derive(Debug, Clone)]
pub struct Location {
    pub source_file: SourceFileHandle,
    pub start_byte: OffsetType,
    pub end_byte: OffsetType,
}