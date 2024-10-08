use crate::infra::source_file::SourceFileHandle;

pub type ByteOffset = i32;

#[derive(Debug, Clone)]
pub struct Location {
    pub source_file: SourceFileHandle,
    pub start_byte: ByteOffset,
    pub end_byte: ByteOffset,
}

impl Location {
    pub fn ephemeral() -> Location {
        Location {
            source_file: SourceFileHandle::from_string("ephemeral", ""),
            start_byte: 0,
            end_byte: 0,
        }
    }
}
