use crate::infra::source_file::SourceFile;

pub type ByteOffset = i32;

#[derive(Debug, Clone)]
pub struct SourceSpan {
    pub source_file: SourceFile,
    pub start_byte: ByteOffset,
    pub end_byte: ByteOffset,
}
const EPHEMERAL_FILE: &str = "<ephemeral file>";

impl SourceSpan {
    pub fn ephemeral() -> SourceSpan {
        SourceSpan {
            source_file: SourceFile::from_string(EPHEMERAL_FILE, ""),
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
