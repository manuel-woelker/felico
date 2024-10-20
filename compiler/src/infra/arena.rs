use crate::infra::full_name::{FullName, FullNameInner};
use crate::model::workspace::WorkspaceString;
use bumpalo::Bump;
use internment::Arena as InternmentArena;
use log::info;
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

    pub fn make_full_name(&self, name: &str) -> FullName<'_> {
        let interned_name = self.intern(name);
        let inner = self.alloc(FullNameInner {
            name_part: interned_name,
            parent: None,
        });
        FullName { inner }
    }
    pub fn make_child_name<'ws>(&'ws self, parent: FullName<'ws>, name: &str) -> FullName<'ws> {
        let interned_name = self.intern(name);
        let inner = self.alloc(FullNameInner {
            name_part: interned_name,
            parent: Some(parent),
        });
        FullName { inner }
    }

    pub fn log_memory_usage(&self) {
        info!(
            "Arena memory usage: {} bytes",
            self.inner.bump.allocated_bytes_including_metadata()
        )
    }
}
