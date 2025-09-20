use felico_base::result::FelicoResult;
use std::fmt::Write;

pub trait TestPrint {
    fn test_print(&self, write: &mut dyn Write, indent: usize) -> FelicoResult<()>;
}
