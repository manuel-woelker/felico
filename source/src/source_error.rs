use crate::source_message::SourceMessage;

#[derive(Debug)]
pub struct SourceError {
    pub source_message: SourceMessage,
}

impl SourceError {
    pub fn new(source_message: SourceMessage) -> Self {
        Self { source_message }
    }
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source_message.render())
    }
}

impl std::error::Error for SourceError {}
