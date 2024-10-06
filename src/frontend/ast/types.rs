use crate::infra::shared_string::SharedString;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

#[derive(Clone)]
pub struct Type {
    inner: Rc<TypeInner>,
}

impl Type {
    pub fn is_unknown(&self) -> bool {
        matches!(self.inner.kind, TypeKind::Unknown)
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

    pub fn tuple(name: &str, components: Vec<Type>) -> Self {
        Self::new(name, TypeKind::Tuple(components))
    }

    pub fn function(name: &str, parameter_types: Vec<Type>, return_type: Type) -> Self {
        Self::new(
            name,
            TypeKind::Function(FunctionType {
                parameter_types,
                return_type,
            }),
        )
    }

    pub fn ty() -> Self {
        Self::new("Type", TypeKind::Type)
    }

    pub fn name(&self) -> &SharedString {
        &self.inner.name
    }
}

impl PartialEq<Self> for Type {
    fn eq(&self, other: &Self) -> bool {
        self.inner.kind == other.inner.kind
    }
}

impl Eq for Type {}

#[derive(Debug)]
pub struct TypeInner {
    name: SharedString,
    kind: TypeKind,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TypeKind {
    Unknown,
    Primitive(PrimitiveType),
    Tuple(Vec<Type>),
    Type,
    Function(FunctionType),
}

#[derive(Debug, Eq, PartialEq)]
pub struct FunctionType {
    pub parameter_types: Vec<Type>,
    pub return_type: Type,
}
#[derive(Debug, Eq, PartialEq)]
pub enum PrimitiveType {
    Bool,
    F64,
    I64,
    Str,
}
