use crate::source_excerpt::SourceExcerpt;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub struct SourceError {
    message: String,
    source_excerpt: SourceExcerpt,
}

impl SourceError {
    pub fn new(message: impl Into<String>, source_excerpt: SourceExcerpt) -> Self {
        Self {
            message: message.into(),
            source_excerpt,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn source_excerpt(&self) -> &SourceExcerpt {
        &self.source_excerpt
    }
}

impl Display for SourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for SourceError {}
