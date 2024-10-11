use crate::infra::diagnostic::InterpreterDiagnostic;
use error_stack::Report;
use std::fmt::{Debug, Display, Formatter};

#[derive(thiserror::Error, Debug)]
pub enum FelicoError {
    #[error("IO Error: {cause}")]
    Io {
        #[from]
        cause: std::io::Error,
    },

    #[error("{0:#?}")]
    Diagnostic(#[from] InterpreterDiagnostic),

    #[error("{message}")]
    Message { message: String },

    #[error("{inner}")]
    Generic {
        inner: Box<dyn std::error::Error + Sync + Send + 'static>,
    },
}

// Wrap error_stack::Report for better encapsulation, and to implement Into Transformations
pub struct FelicoReport {
    pub report: Report<FelicoError>,
}

pub type FelicoResult<T> = Result<T, FelicoReport>;

impl Debug for FelicoReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.report, f)
    }
}

impl Display for FelicoReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.report, f)
    }
}

impl std::error::Error for FelicoReport {}

impl<T: Into<FelicoError>> From<T> for FelicoReport {
    #[track_caller]
    fn from(value: T) -> Self {
        FelicoReport {
            report: Report::new(value.into()),
        }
    }
}

impl From<String> for FelicoError {
    #[track_caller]
    fn from(message: String) -> Self {
        FelicoError::Message { message }
    }
}

impl<'a> From<&'a str> for FelicoError {
    #[track_caller]
    fn from(message: &'a str) -> Self {
        FelicoError::Message {
            message: message.to_string(),
        }
    }
}

pub trait FelicoResultExt<T, E>: Sized {
    #[track_caller]
    fn whatever_context(self, message: &str) -> FelicoResult<T>;
    #[track_caller]
    fn with_whatever_context(self, f: impl FnOnce(&E) -> String) -> FelicoResult<T>;
}

impl<T, E: Into<FelicoReport>> FelicoResultExt<T, E> for Result<T, E> {
    fn whatever_context(self, message: &str) -> FelicoResult<T> {
        self.with_whatever_context(|_| message.to_string())
    }

    #[track_caller]
    fn with_whatever_context(self, f: impl FnOnce(&E) -> String) -> FelicoResult<T> {
        match self {
            Ok(result) => Ok(result),
            Err(e) => {
                let message = f(&e);
                let stack = e.into();
                let stack = FelicoReport {
                    report: stack
                        .report
                        .change_context(FelicoError::Message { message }),
                };
                Err(stack)
            }
        }
    }
}

#[track_caller]
pub fn failed<T: Into<String>>(message: T) -> FelicoReport {
    FelicoError::Message {
        message: message.into(),
    }
    .into()
}

#[allow(unused)]
macro_rules! bail {
    ($($t:tt)*) => {{
        let message = format!($($t)*);
        return Err(crate::infra::result::failed(message));
    }};
}
pub(crate) use bail;

#[cfg(test)]
mod tests {
    use crate::infra::result::FelicoResult;
    use std::io::ErrorKind;

    #[test]
    fn test_io_conversion() {
        fn io() -> Result<(), std::io::Error> {
            Err(std::io::Error::from(ErrorKind::NotFound))
        }
        #[allow(unused)]
        fn outer() -> FelicoResult<()> {
            io()?;
            Ok(())
        }

        let result = outer();
        let err = result.unwrap_err();
        assert_eq!("IO Error: entity not found", format!("{}", err));
    }

    #[test]
    fn test_string_conversion() {
        fn inner() -> Result<(), String> {
            Err("missing entropy".to_string())
        }
        #[allow(unused)]
        fn outer() -> FelicoResult<()> {
            inner()?;
            Ok(())
        }

        let result = outer();
        let err = result.unwrap_err();
        assert_eq!("missing entropy", format!("{}", err));
    }

    #[test]
    fn test_str_conversion() {
        fn inner() -> Result<(), &'static str> {
            Err("missing entropy")
        }
        #[allow(unused)]
        fn outer() -> FelicoResult<()> {
            inner()?;
            Ok(())
        }

        let result = outer();
        let err = result.unwrap_err();
        assert_eq!("missing entropy", format!("{}", err));
    }
}
