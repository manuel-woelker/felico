use crate::infra::shared_string::SharedString;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

#[derive(Clone)]
pub struct Type {
    inner: Rc<TypeInner>,
}

impl Type {
    pub fn is_unknown(&self) -> bool {
        if let TypeKind::Unknown = self.inner.kind {
            true
        } else {
            false
        }
    }
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "❬{}❭", self.inner.name)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "❬{}❭", self.inner.name)
    }
}

impl Type {
    pub fn new<S: Into<SharedString>>(name: S, kind: TypeKind) -> Self {
        Self {
            inner: Rc::new(TypeInner {
                name: name.into(),
                kind,
            }),
        }
    }

    pub fn primitive(name: &str, primitive_type: PrimitiveType) -> Self {
        Self::new(name, TypeKind::Primitive(primitive_type))
    }

    pub fn name(&self) -> &SharedString {
        &self.inner.name
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Type) -> bool {
        self.inner.kind == other.inner.kind
    }
}

#[derive(Debug)]
pub struct TypeInner {
    name: SharedString,
    kind: TypeKind,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TypeKind {
    Unknown,
    Primitive(PrimitiveType),
}

#[derive(Debug, Eq, PartialEq)]
pub enum PrimitiveType {
    Bool,
    Unit,
    F64,
    String,
}
