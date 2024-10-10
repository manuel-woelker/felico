use crate::infra::source_file::SourceFileHandle;

pub type ByteOffset = i32;

#[derive(Debug, Clone)]
pub struct Location {
    pub source_file: SourceFileHandle,
    pub start_byte: ByteOffset,
    pub end_byte: ByteOffset,
}
const EPHEMERAL_FILE: &'static str = "<ephemeral file>";

impl Location {
    pub fn ephemeral() -> Location {
        Location {
            source_file: SourceFileHandle::from_string(EPHEMERAL_FILE, ""),
            start_byte: 0,
            end_byte: 0,
        }
    }

    pub fn is_ephemeral(&self) -> bool {
        self.source_file.filename() == EPHEMERAL_FILE
    }
}
