use std::backtrace::{Backtrace, BacktraceStatus};
use std::fmt::{Debug, Formatter};

mod message_error;
use crate::unansi;
pub use message_error::MessageError;

pub struct FelicoError {
    pub error: Box<dyn std::error::Error>,
    pub backtrace: Backtrace,
}

impl Debug for FelicoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.write_to(f)?;
        if self.backtrace.status() == BacktraceStatus::Captured {
            writeln!(f, "{}", self.backtrace)?;
        }
        Ok(())
    }
}

impl FelicoError {
    pub fn message(s: impl Into<String>) -> Self {
        MessageError::from(s).into()
    }

    pub fn write_to(&self, write: &mut dyn std::fmt::Write) -> std::fmt::Result {
        writeln!(write, "Error: {}", self.error)?;
        Ok(())
    }
}

impl<T> From<T> for FelicoError
where
    T: std::error::Error + 'static,
{
    fn from(value: T) -> Self {
        Self {
            error: Box::new(value),
            backtrace: Backtrace::capture(),
        }
    }
}

impl FelicoError {
    pub fn to_test_string(&self) -> String {
        let mut test_string = String::new();
        self.write_to(&mut test_string).unwrap();
        unansi(&test_string)
    }
}

#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {
        $crate::error::FelicoError::message(format!($($arg)*))
    };
}
pub use err;

#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::err!($($arg)*))
    };
}
pub use bail;

#[cfg(test)]
mod tests {
    use crate::result::FelicoResult;

    #[test]
    fn test_err() {
        let err = err!("test {}", 123);
        assert_eq!(err.error.to_string(), "test 123");
    }

    #[test]
    fn test_bail() {
        let err = (|| -> FelicoResult<()> {
            bail!("test {}", 123);
        })()
        .unwrap_err();
        assert_eq!(err.error.to_string(), "test 123");
    }
    /*
       use expect_test::expect;
       use crate::error::FelicoError;
       use crate::result::FelicoResult;

       #[inline(never)]
       fn foo() -> FelicoResult<()> {
           bar()
       }

       #[inline(never)]
       fn bar() -> FelicoResult<()> {
           Err(FelicoError::message("bar"))
       }

       #[test]
       fn test_backtrace() {
           let result = foo().expect_err("foo should fail");
           expect!([r#"
                Error: bar

            "#]).assert_debug_eq(&result);
       }

    */
}
