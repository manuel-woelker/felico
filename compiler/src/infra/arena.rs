use crate::infra::full_name::{FullName, FullNameInner};
use crate::model::workspace::WorkspaceString;
use bumpalo::Bump;
use internment::Arena as InternmentArena;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

pub struct ArenaInner {
    bump: Bump,
    string_arena: InternmentArena<str>,
}

#[derive(Clone)]
pub struct Arena {
    inner: Rc<ArenaInner>,
}

impl Debug for Arena {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Arena")
    }
}

impl Arena {
    pub fn new() -> Self {
        let inner = ArenaInner {
            bump: Bump::new(),
            string_arena: internment::Arena::new(),
        };
        Self {
            inner: Rc::new(inner),
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

    pub fn make_full_name(&self, string: &str) -> FullName<'_> {
        let interned_string = self.intern(string);
        let inner = self.alloc(FullNameInner {
            name_part: interned_string,
            parent: None,
        });
        FullName { inner }
    }
}
