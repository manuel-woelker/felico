use std::fmt::{Debug, Formatter};

pub struct SourceFile {
    path: String,
    content: String,
}

impl SourceFile {
    pub fn new(path: String, content: String) -> Self {
        Self { path, content }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Debug for SourceFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceFile")
            .field("path", &self.path)
            .finish()
    }
}
