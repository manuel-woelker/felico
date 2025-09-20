use std::fmt::{Debug, Formatter};

mod message_error;
pub use message_error::MessageError;
mod source_error;
pub use source_error::SourceError;

pub struct FelicoError {
    pub error: Box<dyn std::error::Error>,
}

impl Debug for FelicoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.error, f)
    }
}

impl FelicoError {
    pub fn message(s: impl Into<String>) -> Self {
        Self {
            error: Box::new(MessageError::from(s)),
        }
    }
}

impl<T> From<T> for FelicoError
where
    T: std::error::Error + 'static,
{
    fn from(value: T) -> Self {
        Self {
            error: Box::new(value),
        }
    }
}
