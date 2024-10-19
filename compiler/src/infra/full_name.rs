use crate::infra::shared_string::Name;
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone)]
pub struct FullName<'ws> {
    pub(crate) inner: &'ws FullNameInner<'ws>,
}

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
    /*    pub fn new(name_part: Name<'ws>, parent: Option<FullName<'ws>>) -> Self {
        if name_part.is_empty() {
            panic!("name part is empty");
        }
        Self {
            inner: &FullNameInner { name_part, parent }),
        }
    }*/

    pub fn short_name(&self) -> &str {
        self.inner.name_part
    }
    /*
    pub fn child<S: Into<Name>>(&self, name: S) -> Self {
        Self::new(name.into(), Some(self.clone()))
    }*/
}
/*
impl<S: Into<Name>> From<S> for FullName {
    fn from(name: S) -> Self {
        Self::new(name.into(), None)
    }
}
*/
#[cfg(test)]
mod tests {
    /*

    fn assert_name(name: &FullName, expected: &str) {
        assert_eq!(format!("{}", name), expected);
        assert_eq!(format!("{:?}", name), expected);
        let short_name = expected.rsplit_once("::").map(|x| x.1).unwrap_or(expected);
        assert_eq!(name.short_name(), short_name);
    }
    #[test]
    fn display_simple_name() {
        let name: FullName = "foo".into();
        assert_name(&name, "foo");
    }*/
    /*
        #[test]
        fn display_complex_name() {
            let root: FullName = "foo".into();
            let child = root.child("bar");
            assert_name(&child, "foo::bar");

            let grand_child = child.child("baz");
            assert_name(&grand_child, "foo::bar::baz");
        }
    */
    #[test]
    fn equals() {
        /*
        fn assert_eq(a: &FullName, b: &FullName) {
            assert_eq!(a, b);
            assert_eq!(b, a);
        }
        fn assert_ne(a: &FullName, b: &FullName) {
            assert_ne!(a, b);
            assert_ne!(b, a);
        }

        let root: FullName = "foo".into();
        let root2: FullName = "foo".into();
        assert_eq(&root, &root);
        assert_eq(&root, &root2);
        assert_eq(&root2, &root);
        let child = root.child("bar");
        let child2 = root2.child("bar");
        let child3 = root.child("bar");

        assert_eq(&child, &child);
        assert_eq(&child, &child2);
        assert_eq(&child, &child3);

        let grand_child = child.child("baz");
        let grand_child2 = child2.child("baz");
        assert_eq(&grand_child, &grand_child2);

        assert_ne(&root, &child);
        assert_ne(&root, &grand_child);
        assert_ne(&child, &grand_child);

        let other_root: FullName = "other".into();
        assert_ne(&root, &other_root);

        let other_child = other_root.child("bar");
        let other_child2 = root.child("other");

        assert_ne(&other_child, &child);
        assert_ne(&other_child2, &child);
        assert_ne(&other_child, &other_child2);*/
    }
    /*
    #[test]
    fn assert_not_empty() {
        let err = catch_unwind_silent(|| {
            let _name = FullName::from("");
        })
        .unwrap_err();
        assert_eq!(*err.downcast::<&str>().unwrap(), "name part is empty");
    }*/
}
