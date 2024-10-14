use crate::frontend::lex::token::Token;
use crate::infra::shared_string::SharedString;
use crate::infra::source_span::SourceSpan;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

#[derive(Clone)]
pub struct Type {
    inner: Arc<TypeInner>,
}

impl Type {
    pub fn is_unknown(&self) -> bool {
        matches!(self.inner.kind, TypeKind::Unknown)
    }
    pub fn is_bool(&self) -> bool {
        matches!(self.inner.kind, TypeKind::Primitive(PrimitiveType::Bool))
    }
    pub fn kind(&self) -> &TypeKind {
        &self.inner.kind
    }

    pub fn declaration_site(&self) -> &SourceSpan {
        &self.inner.declaration_site
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
    pub fn new<S: Into<SharedString>>(
        name: S,
        kind: TypeKind,
        declaration_site: SourceSpan,
    ) -> Self {
        Self {
            inner: Arc::new(TypeInner {
                name: name.into(),
                kind,
                declaration_site,
            }),
        }
    }

    pub fn new_ephemeral<S: Into<SharedString>>(name: S, kind: TypeKind) -> Self {
        Self::new(name, kind, SourceSpan::ephemeral())
    }

    pub fn primitive(name: &str, primitive_type: PrimitiveType) -> Self {
        Self::new_ephemeral(name, TypeKind::Primitive(primitive_type))
    }

    pub fn function(
        name: &str,
        parameter_types: Vec<Type>,
        return_type: Type,
        declaration_site: SourceSpan,
    ) -> Self {
        Self::new(
            name,
            TypeKind::Function(FunctionType {
                parameter_types,
                return_type,
            }),
            declaration_site,
        )
    }

    pub fn ty() -> Self {
        Self::new_ephemeral("Type", TypeKind::Type)
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
    declaration_site: SourceSpan,
    kind: TypeKind,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TypeKind {
    Any,   // Top Type, should only be used for debug_print()
    Never, // Bottom Type, the type of return expressions and return type of divergent functions
    Unknown,
    Unresolved, // failed to resolve
    Primitive(PrimitiveType),
    Type,
    Function(FunctionType),
    Struct(StructType),
}

impl Eq for StructType {}
impl PartialEq for StructType {
    fn eq(&self, other: &Self) -> bool {
        self.name.lexeme() == other.name.lexeme()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct FunctionType {
    pub parameter_types: Vec<Type>,
    pub return_type: Type,
}

#[derive(Debug)]
pub struct StructType {
    pub name: Token,
    pub fields: HashMap<SharedString, StructField>,
}

#[derive(Debug)]
pub struct StructField {
    pub name_token: Token,
    pub name: SharedString,
    pub ty: Type,
}

impl StructField {
    pub fn new(name_token: &Token, ty: &Type) -> Self {
        Self {
            name: SharedString::from(name_token.lexeme()),
            name_token: name_token.clone(),
            ty: ty.clone(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum PrimitiveType {
    Bool,
    F64,
    I64,
    Str,
}
