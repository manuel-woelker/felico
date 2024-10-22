use crate::infra::shared_string::{Name, SharedString};
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone)]
pub struct FullName<'ws> {
    pub(crate) inner: &'ws FullNameInner<'ws>,
}

const UNRESOLVED_FULL_NAME: FullName<'static> = FullName {
    inner: &FullNameInner {
        name_part: "<unresolved>",
        parent: None,
    },
};

impl<'ws> Copy for FullName<'ws> {}

pub struct FullNameInner<'ws> {
    pub name_part: Name<'ws>,
    pub parent: Option<FullName<'ws>>,
}

impl<'ws> Debug for FullName<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl<'ws> Display for FullName<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(parent) = &self.inner.parent {
            Debug::fmt(parent, f)?;
            write!(f, "::")?;
        }
        f.write_str(self.inner.name_part)
    }
}

impl<'ws> PartialEq<Self> for FullName<'ws> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.name_part == other.inner.name_part && self.inner.parent == other.inner.parent
    }
}

impl<'ws> PartialEq<str> for FullName<'ws> {
    #[allow(clippy::cmp_owned)]
    fn eq(&self, other: &str) -> bool {
        self.to_string() == other
    }
}

impl<'ws> Eq for FullName<'ws> {}

impl<'ws> From<&FullName<'ws>> for String {
    fn from(value: &FullName) -> String {
        value.to_string()
    }
}

impl<'ws> FullName<'ws> {
    pub fn short_name(&self) -> &'ws str {
        self.inner.name_part
    }

    pub fn unresolved() -> Self {
        UNRESOLVED_FULL_NAME
    }
    pub fn is_unresolved(&self) -> bool {
        self == &UNRESOLVED_FULL_NAME
    }

    pub fn parts(&self) -> Vec<SharedString<'ws>> {
        let mut parts = Vec::new();
        parts.push(self.short_name());
        let mut maybe_parent = self.inner.parent;
        while let Some(parent) = maybe_parent {
            parts.push(parent.short_name());
            maybe_parent = parent.inner.parent
        }
        parts.reverse();
        parts
    }
}

#[cfg(test)]
mod tests {
    use crate::infra::arena::Arena;
    use crate::infra::full_name::FullName;
    use crate::infra::test_util::catch_unwind_silent;

    fn assert_name(name: &FullName, expected: &str) {
        assert_eq!(format!("{}", name), expected);
        assert_eq!(format!("{:?}", name), expected);
        let short_name = expected.rsplit_once("::").map(|x| x.1).unwrap_or(expected);
        assert_eq!(name.short_name(), short_name);
    }
    #[test]
    fn display_simple_name() {
        let arena = Arena::new();
        let name: FullName = arena.make_full_name("foo");
        assert_name(&name, "foo");
    }
    #[test]
    fn display_complex_name() {
        let arena = Arena::new();
        let root: FullName = arena.make_full_name("foo");
        let child = arena.make_child_name(root, "bar");
        assert_name(&child, "foo::bar");

        let grand_child = arena.make_child_name(child, "baz");
        assert_name(&grand_child, "foo::bar::baz");
    }
    #[test]
    fn equals() {
        fn assert_eq(a: &FullName, b: &FullName) {
            assert_eq!(a, b);
            assert_eq!(b, a);
        }
        fn assert_ne(a: &FullName, b: &FullName) {
            assert_ne!(a, b);
            assert_ne!(b, a);
        }
        let arena = Arena::new();

        let root: FullName = arena.make_full_name("foo");
        let root2: FullName = arena.make_full_name("foo");
        assert_eq(&root, &root);
        assert_eq(&root, &root2);
        assert_eq(&root2, &root);
        let child = arena.make_child_name(root, "bar");
        let child2 = arena.make_child_name(root2, "bar");
        let child3 = arena.make_child_name(root, "bar");

        assert_eq(&child, &child);
        assert_eq(&child, &child2);
        assert_eq(&child, &child3);

        let grand_child = arena.make_child_name(child, "baz");
        let grand_child2 = arena.make_child_name(child2, "baz");
        assert_eq(&grand_child, &grand_child2);

        assert_ne(&root, &child);
        assert_ne(&root, &grand_child);
        assert_ne(&child, &grand_child);

        let other_root: FullName = arena.make_full_name("other");
        assert_ne(&root, &other_root);

        let other_child = arena.make_child_name(other_root, "bar");
        let other_child2 = arena.make_child_name(root, "other");

        assert_ne(&other_child, &child);
        assert_ne(&other_child2, &child);
        assert_ne(&other_child, &other_child2);
    }
    #[test]
    fn assert_not_empty() {
        let err = catch_unwind_silent(|| {
            let arena = Arena::new();
            let _name = arena.make_full_name("");
        })
        .unwrap_err();
        assert_eq!(*err.downcast::<&str>().unwrap(), "name cannot be empty");
    }
}
