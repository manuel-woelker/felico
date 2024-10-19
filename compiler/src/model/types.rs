use crate::frontend::lex::token::Token;
use crate::infra::full_name::FullName;
use crate::infra::shared_string::SharedString;
use crate::infra::source_span::SourceSpan;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone)]
pub struct Type<'ws> {
    pub(crate) inner: &'ws TypeInner<'ws>,
}

impl<'ws> Type<'ws> {
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

impl<'ws> Debug for Type<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "❬{}❭", self.inner.name)
    }
}

impl<'ws> Display for Type<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "❬{}❭", self.inner.name)
    }
}
/*
impl <'ws> Type {
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
impl<'ws> PartialEq<Self> for Type<'ws> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.kind == other.inner.kind
    }
}

impl<'ws> Eq for Type<'ws> {}

#[derive(Debug)]
pub struct TypeInner<'ws> {
    pub name: FullName,
    pub declaration_site: SourceSpan<'ws>,
    pub kind: TypeKind<'ws>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TypeKind<'ws> {
    Any,   // Top Type, should only be used for debug_print()
    Never, // Bottom Type, the type of return expressions and return type of divergent functions
    Unknown,
    Unresolved, // failed to resolve
    Primitive(PrimitiveType),
    Type,
    Namespace,
    Function(FunctionType<'ws>),
    Struct(StructType<'ws>),
    Trait(TraitType<'ws>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct FunctionType<'ws> {
    pub parameter_types: Vec<Type<'ws>>,
    pub return_type: Type<'ws>,
}

#[derive(Debug)]
pub struct StructType<'ws> {
    pub name: Token<'ws>,
    pub fields: HashMap<SharedString, StructField<'ws>>,
}

impl<'ws> Eq for StructType<'ws> {}
impl<'ws> PartialEq for StructType<'ws> {
    fn eq(&self, other: &Self) -> bool {
        self.name.lexeme() == other.name.lexeme()
    }
}

#[derive(Debug)]
pub struct TraitType<'ws> {
    pub name: Token<'ws>,
}

impl<'ws> Eq for TraitType<'ws> {}
impl<'ws> PartialEq for TraitType<'ws> {
    fn eq(&self, other: &Self) -> bool {
        self.name.lexeme() == other.name.lexeme()
    }
}

#[derive(Debug)]
pub struct StructField<'ws> {
    pub name_token: Token<'ws>,
    pub name: SharedString,
    pub ty: Type<'ws>,
}

impl<'ws> StructField<'ws> {
    pub fn new(name_token: &Token<'ws>, ty: Type<'ws>) -> Self {
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
