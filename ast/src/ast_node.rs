use crate::test_print::TestPrint;
use felico_base::indent;
use felico_base::result::FelicoResult;
use felico_source::file_location::FileLocation;
use std::fmt::Write;

pub struct AstNode<'source, T: TestPrint> {
    pub location: FileLocation<'source>,
    pub node: T,
}

impl<'source, T: TestPrint> AstNode<'source, T> {
    pub fn new(location: FileLocation<'source>, node: T) -> Self {
        Self { location, node }
    }
}

impl<'source, T: TestPrint> TestPrint for AstNode<'source, T> {
    fn test_print(&self, write: &mut dyn Write, indent: usize) -> FelicoResult<()> {
        write!(
            write,
            "🌲 {:3}+{:<3}",
            self.location.start,
            self.location.end - self.location.start
        )?;
        indent::indent(write, indent)?;
        self.node.test_print(write, indent)
    }
}
