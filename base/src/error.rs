use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct FelicoError {
    pub error: Box<dyn std::error::Error>,
}

impl Debug for FelicoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.error, f)
    }
}

pub struct MessageError {
    message: String,
}

impl Debug for MessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageError")
            .field("message", &self.message)
            .finish()
    }
}

impl Display for MessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for MessageError {}

impl FelicoError {
    pub fn message(s: impl Into<String>) -> Self {
        Self {
            error: Box::new(MessageError { message: s.into() }),
        }
    }
}
