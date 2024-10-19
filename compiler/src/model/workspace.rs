use crate::infra::source_file::{SourceFile, SourceFileInner};
use bumpalo::Bump;
use internment::Arena;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

#[derive(Clone)]
pub struct Workspace {
    inner: Rc<WorkspaceInner>,
}

impl Debug for Workspace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Workspace")
    }
}

struct WorkspaceInner {
    bump: Bump,
    string_arena: Arena<str>,
}

pub type WorkspaceString<'ws> = &'ws str;

impl Workspace {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(WorkspaceInner {
                bump: Bump::new(),
                string_arena: Arena::new(),
            }),
        }
    }

    pub fn alloc<T>(&self, val: T) -> &mut T {
        self.inner.bump.alloc(val)
    }

    pub fn intern(&self, string: &str) -> WorkspaceString<'_> {
        self.inner.string_arena.intern(string).into_ref()
    }

    pub fn alloc_str(&self, string: &str) -> WorkspaceString<'_> {
        self.inner.bump.alloc_str(string)
    }

    pub fn source_file_from_string<F: AsRef<str>, S: AsRef<str>>(
        &self,
        filename: F,
        source_code: S,
    ) -> SourceFile<'_> {
        let ws_filename = self.alloc_str(filename.as_ref());
        let ws_source_code = self.alloc_str(source_code.as_ref());
        let inner = self.alloc(SourceFileInner {
            filename: ws_filename,
            source_code: ws_source_code,
        });
        SourceFile { inner }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::workspace::{Workspace, WorkspaceString};

    pub struct Module {
        _id: usize,
    }

    fn create_module() -> Module {
        Module { _id: 0 }
    }

    #[test]
    fn test_basic() {
        let workspace = Workspace::new();
        let module_bar = workspace.alloc(create_module()) as &Module;
        let module_bar2 = module_bar;
        let module_foo = workspace.alloc(create_module()) as &Module;
        assert_eq!(module_bar as *const _, module_bar2 as *const _);
        assert_ne!(module_bar as *const _, module_foo as *const _);
    }

    #[test]
    fn test_intern() {
        let workspace = Workspace::new();
        let string_foo = workspace.intern("foo");
        let string_foo2 = workspace.intern("foo");
        assert_eq!(string_foo, string_foo2);
        assert_eq!(string_foo as *const _, string_foo2 as *const _);
    }

    #[test]
    fn test_alloc_str() {
        let workspace = Workspace::new();
        let string_foo = workspace.alloc_str("foo");
        let string_foo2 = string_foo;
        assert_eq!(string_foo, string_foo2);
        assert_eq!(string_foo as *const _, string_foo2 as *const _);
    }

    struct Tester<'ws> {
        pub workspace: &'ws Workspace,
        pub strings: Vec<&'ws str>,
    }

    impl<'ws> Tester<'ws> {
        // Note to self: this "&mut self" reference must NOT have a 'ws lifetime, since this causes the borrow to live too long
        pub fn add_string(&mut self) -> WorkspaceString<'ws> {
            let string = self.workspace.alloc_str("xyz");
            self.strings.push(string);
            string
        }
    }

    #[test]
    fn test_workspace() {
        let workspace = Workspace::new();
        let mut tester = Tester {
            workspace: &workspace,
            strings: vec![],
        };
        let string_foo = tester.workspace.alloc_str("foo");
        let string_xyz = tester.add_string();
        tester.strings.push(string_foo);
        assert_eq!(string_foo, tester.strings[1]);
        assert_eq!(string_xyz, tester.strings[0]);
    }
}
