use std::fmt::{Display, Formatter};

pub enum Value {
    String(String),
}

impl Value {
    pub fn string(string: impl Into<String>) -> Self {
        Self::String(string.into())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(string) => write!(f, "\"{string}\""),
        }
    }
}
