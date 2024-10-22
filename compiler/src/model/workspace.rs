use crate::frontend::lex::token::{Token, TokenType};
use crate::infra::arena::Arena;
use crate::infra::full_name::{FullName, FullNameInner};
use crate::infra::result::FelicoResult;
use crate::infra::source_file::{SourceFile, SourceFileInner};
use crate::infra::source_span::SourceSpan;
use crate::interpret::value::ValueFactory;
use crate::model::type_factory::TypeFactory;
use std::fmt::{Debug, Formatter};
use std::path::Path;

#[derive(Clone)]
pub struct Workspace<'ws> {
    inner: &'ws WorkspaceInner<'ws>,
}

impl<'ws> Copy for Workspace<'ws> {}

impl<'ws> Debug for Workspace<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Workspace")
    }
}

pub struct WorkspaceInner<'ws> {
    pub arena: &'ws Arena,
    pub type_factory: TypeFactory<'ws>,
    pub value_factory: ValueFactory<'ws>,
    pub unresolved_full_name: FullName<'ws>,
}

pub type WorkspaceString<'ws> = &'ws str;

impl<'ws> Workspace<'ws> {
    pub fn new(arena: &'ws Arena) -> Self {
        let unresolved_full_name = FullName {
            inner: arena.alloc(FullNameInner {
                name_part: "unresolved",
                parent: None,
            }),
        };
        let type_factory = TypeFactory::new(arena);
        let value_factory = ValueFactory::new(type_factory, arena);
        let inner = WorkspaceInner {
            arena,
            type_factory,
            value_factory,
            unresolved_full_name,
        };
        Self {
            inner: arena.alloc(inner),
        }
    }

    pub fn type_factory(&self) -> TypeFactory<'ws> {
        self.inner.type_factory
    }

    pub fn value_factory(&self) -> ValueFactory<'ws> {
        self.inner.value_factory
    }

    pub fn alloc<T>(&self, val: T) -> &'ws mut T {
        self.inner.arena.alloc(val)
    }

    pub fn alloc_str(&self, string: &str) -> WorkspaceString<'ws> {
        self.inner.arena.alloc_str(string)
    }

    pub fn intern(&self, string: &str) -> WorkspaceString<'ws> {
        self.inner.arena.intern(string)
    }

    pub fn make_full_name(&self, string: &str) -> FullName<'ws> {
        self.inner.arena.make_full_name(string)
    }

    pub fn make_child_name(&self, parent: FullName<'ws>, name: &str) -> FullName<'ws> {
        self.inner.arena.make_child_name(parent, name)
    }

    pub fn source_file_from_string<F: AsRef<str>, S: AsRef<str>>(
        &self,
        filename: F,
        source_code: S,
    ) -> SourceFile<'ws> {
        let ws_filename = self.alloc_str(filename.as_ref());
        let ws_source_code = self.alloc_str(source_code.as_ref());
        let inner = self.alloc(SourceFileInner {
            filename: ws_filename,
            source_code: ws_source_code,
        });
        SourceFile { inner }
    }

    pub fn source_file_from_path<P: AsRef<Path>>(&self, path: P) -> FelicoResult<SourceFile<'ws>> {
        let actual_path = path.as_ref();
        let filename = actual_path.to_str().ok_or(format!(
            "Could not turn path into string: {:?}",
            path.as_ref()
        ))?;
        let file = std::fs::File::open(actual_path)?;
        let source = std::io::read_to_string(file)?;
        Ok(self.source_file_from_string(filename, source))
    }

    pub(crate) fn make_token(
        &self,
        token_type: TokenType,
        location: SourceSpan<'ws>,
        value: &'ws str,
    ) -> Token<'ws> {
        self.inner.arena.make_token(token_type, location, value)
    }
}

#[cfg(test)]
mod tests {
    use crate::infra::arena::Arena;
    use crate::model::workspace::{Workspace, WorkspaceString};

    pub struct Module {
        _id: usize,
    }

    fn create_module() -> Module {
        Module { _id: 0 }
    }

    #[test]
    fn test_basic() {
        let arena = Arena::new();
        let workspace = Workspace::new(&arena);
        let module_bar = workspace.alloc(create_module()) as &Module;
        let module_bar2 = module_bar;
        let module_foo = workspace.alloc(create_module()) as &Module;
        assert_eq!(module_bar as *const _, module_bar2 as *const _);
        assert_ne!(module_bar as *const _, module_foo as *const _);
    }

    #[test]
    fn test_intern() {
        let arena = Arena::new();
        let workspace = Workspace::new(&arena);
        let string_foo = workspace.intern("foo");
        let string_foo2 = workspace.intern("foo");
        assert_eq!(string_foo, string_foo2);
        assert_eq!(string_foo as *const _, string_foo2 as *const _);
    }

    #[test]
    fn test_alloc_str() {
        let arena = Arena::new();
        let workspace = Workspace::new(&arena);
        let string_foo = workspace.alloc_str("foo");
        let string_foo2 = string_foo;
        assert_eq!(string_foo, string_foo2);
        assert_eq!(string_foo as *const _, string_foo2 as *const _);
    }

    struct Tester<'ws> {
        pub workspace: &'ws Workspace<'ws>,
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
        let arena = Arena::new();
        let workspace = Workspace::new(&arena);
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
