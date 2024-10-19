use crate::frontend::lex::token::Token;
use crate::infra::full_name::FullName;
use crate::infra::shared_string::SharedString;
use crate::infra::source_span::SourceSpan;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone)]
pub struct Type<'a> {
    pub(crate) inner: &'a TypeInner<'a>,
}

impl<'a> Type<'a> {
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

impl<'a> Debug for Type<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "❬{}❭", self.inner.name)
    }
}

impl<'a> Display for Type<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "❬{}❭", self.inner.name)
    }
}
/*
impl <'a> Type {
    pub fn new<S: Into<FullName>>(name: S, kind: TypeKind, declaration_site: SourceSpan) -> Self {
        Self {
            inner: Rc::new(TypeInner {
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

    pub fn name(&self) -> &FullName {
        &self.inner.name
    }
    pub fn short_name(&self) -> &str {
        self.inner.name.short_name()
    }
}


 */
impl<'a> PartialEq<Self> for Type<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.kind == other.inner.kind
    }
}

impl<'a> Eq for Type<'a> {}

#[derive(Debug)]
pub struct TypeInner<'a> {
    pub name: FullName,
    pub declaration_site: SourceSpan<'a>,
    pub kind: TypeKind<'a>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TypeKind<'a> {
    Any,   // Top Type, should only be used for debug_print()
    Never, // Bottom Type, the type of return expressions and return type of divergent functions
    Unknown,
    Unresolved, // failed to resolve
    Primitive(PrimitiveType),
    Type,
    Namespace,
    Function(FunctionType<'a>),
    Struct(StructType<'a>),
    Trait(TraitType<'a>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct FunctionType<'a> {
    pub parameter_types: Vec<Type<'a>>,
    pub return_type: Type<'a>,
}

#[derive(Debug)]
pub struct StructType<'a> {
    pub name: Token<'a>,
    pub fields: HashMap<SharedString, StructField<'a>>,
}

impl<'a> Eq for StructType<'a> {}
impl<'a> PartialEq for StructType<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name.lexeme() == other.name.lexeme()
    }
}

#[derive(Debug)]
pub struct TraitType<'a> {
    pub name: Token<'a>,
}

impl<'a> Eq for TraitType<'a> {}
impl<'a> PartialEq for TraitType<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name.lexeme() == other.name.lexeme()
    }
}

#[derive(Debug)]
pub struct StructField<'a> {
    pub name_token: Token<'a>,
    pub name: SharedString,
    pub ty: Type<'a>,
}

impl<'a> StructField<'a> {
    pub fn new(name_token: &Token<'a>, ty: Type<'a>) -> Self {
        Self {
            name: SharedString::from(name_token.lexeme()),
            name_token: name_token.clone(),
            ty,
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
