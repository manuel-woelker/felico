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

pub type WorkspaceString<'a> = &'a str;

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
}

#[cfg(test)]
mod tests {
    use crate::model::workspace::Workspace;

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
}
