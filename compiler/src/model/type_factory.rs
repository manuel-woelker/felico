use crate::frontend::lex::token::{Token, TokenType};
use crate::infra::arena::Arena;
use crate::infra::shared_string::SharedString;
use crate::infra::source_span::SourceSpan;
use crate::model::types::{
    FunctionType, PrimitiveType, StructField, StructType, TraitType, Type, TypeInner, TypeKind,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct TypeFactory<'ws> {
    inner: &'ws TypeFactoryInner<'ws>,
}

impl<'ws> Copy for TypeFactory<'ws> {}

macro_rules! factory_fns {
        ($($id:ident),+) => {
    #[derive(Debug)]
struct TypeFactoryInner<'ws> {
            arena: &'ws Arena,
            $(
            $id: Type<'ws>,
            )+
}


    impl <'ws> TypeFactory<'ws>  {
            $(
            pub fn $id(&self) -> Type<'ws> {
                self.inner.$id.clone()
            }
            )+
    }
    }

}

factory_fns!(bool, unit, i64, f64, ty, str, unknown, unresolved, never);
//factory_fns!(bool);

impl<'ws> TypeFactory<'ws> {
    pub fn new(arena: &'ws Arena) -> TypeFactory<'ws> {
        let make_type = |name: &str, kind: TypeKind<'ws>| -> Type<'ws> {
            Type {
                inner: arena.alloc(TypeInner {
                    name: arena.make_full_name(name),
                    kind,
                    declaration_site: SourceSpan::ephemeral(),
                }),
            }
        };

        Self {
            inner: arena.alloc(TypeFactoryInner {
                arena,
                bool: make_type("bool", TypeKind::Primitive(PrimitiveType::Bool)),
                unit: make_type(
                    "Unit",
                    TypeKind::Struct(StructType {
                        name: Token {
                            token_type: TokenType::Identifier,
                            location: SourceSpan::ephemeral(),
                            value: "Unit",
                        },
                        fields: Default::default(),
                    }),
                ),
                f64: make_type("f64", TypeKind::Primitive(PrimitiveType::F64)),
                i64: make_type("i64", TypeKind::Primitive(PrimitiveType::I64)),
                str: make_type("str", TypeKind::Primitive(PrimitiveType::Str)),
                ty: make_type("Type", TypeKind::Type),
                unknown: make_type("unknown", TypeKind::Unknown),
                never: make_type("never", TypeKind::Never),
                unresolved: make_type("unresolved", TypeKind::Unresolved),
            }),
        }
    }

    pub fn make_type<S: AsRef<str>>(
        &self,
        name: S,
        kind: TypeKind<'ws>,
        declaration_site: SourceSpan<'ws>,
    ) -> Type<'ws> {
        let ws_name = self.inner.arena.make_full_name(name.as_ref());
        Type {
            inner: self.inner.arena.alloc(TypeInner {
                name: ws_name,
                kind,
                declaration_site,
            }),
        }
    }

    pub fn function(
        &self,
        parameter_types: Vec<Type<'ws>>,
        return_type: Type<'ws>,
        declaration_site: SourceSpan<'ws>,
    ) -> Type<'ws> {
        let name = "Fn(".to_string()
            + &parameter_types
                .iter()
                .map(|ty| ty.to_string())
                .collect::<Vec<String>>()
                .join(", ")
            + ") -> "
            + &return_type.to_string();
        self.make_type(
            &name,
            TypeKind::Function(FunctionType {
                parameter_types,
                return_type,
            }),
            declaration_site,
        )
    }

    pub fn make_struct(
        &'ws self,
        name: &Token<'ws>,
        fields: HashMap<SharedString<'ws>, StructField<'ws>>,
        declaration_site: SourceSpan<'ws>,
    ) -> Type<'ws> {
        self.make_type(
            name.lexeme(),
            TypeKind::Struct(StructType {
                name: name.clone(),
                fields,
            }),
            declaration_site,
        )
    }

    pub fn make_namespace(
        &self,
        name: &Token<'ws>,
        //        symbol_map: HashMap<SharedString, InterpreterValue>,
        declaration_site: SourceSpan<'ws>,
    ) -> Type {
        self.make_type(name.lexeme(), TypeKind::Namespace, declaration_site)
    }

    pub fn make_trait(&self, name: &Token<'ws>, declaration_site: SourceSpan<'ws>) -> Type {
        self.make_type(
            name.lexeme(),
            TypeKind::Trait(TraitType { name: name.clone() }),
            declaration_site,
        )
    }

    pub fn make_ephemeral<S: AsRef<str>>(&'ws self, name: S, kind: TypeKind<'ws>) -> Type<'ws> {
        self.make_type(name, kind, SourceSpan::ephemeral())
    }
}
