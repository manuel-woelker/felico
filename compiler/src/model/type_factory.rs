use crate::frontend::lex::token::{Token, TokenType};
use crate::infra::shared_string::SharedString;
use crate::infra::source_span::SourceSpan;
use crate::model::types::{PrimitiveType, StructField, StructType, TraitType, Type, TypeKind};
use crate::model::workspace::Workspace;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct TypeFactory {
    inner: Rc<TypeFactoryInner>,
}

macro_rules! factory_fns {
        ($($id:ident),+) => {
    #[derive(Debug)]
struct TypeFactoryInner {
            workspace: Workspace,
            $(
            $id: Type,
            )+
}


    impl TypeFactory {
            $(
            pub fn $id(&self) -> Type {
                self.inner.$id.clone()
            }
            )+
    }
    }

}

factory_fns!(bool, unit, i64, f64, ty, str, unknown, unresolved, never);

impl TypeFactory {
    pub fn new(workspace: &Workspace) -> Self {
        Self {
            inner: Rc::new(TypeFactoryInner {
                workspace: workspace.clone(),
                bool: Type::primitive("bool", PrimitiveType::Bool),
                unit: Type::new_ephemeral(
                    "Unit",
                    TypeKind::Struct(StructType {
                        name: Token {
                            token_type: TokenType::Identifier,
                            location: SourceSpan::ephemeral(),
                            value: Some(SharedString::from("Unit")),
                        },
                        fields: Default::default(),
                    }),
                ),
                f64: Type::primitive("f64", PrimitiveType::F64),
                i64: Type::primitive("i64", PrimitiveType::I64),
                str: Type::primitive("str", PrimitiveType::Str),
                ty: Type::ty(),
                unknown: Type::new_ephemeral("unknown", TypeKind::Unknown),
                never: Type::new_ephemeral("never", TypeKind::Never),
                unresolved: Type::new_ephemeral("unresolved", TypeKind::Unresolved),
            }),
        }
    }

    pub fn function(
        &self,
        parameter_types: Vec<Type>,
        return_type: Type,
        declaration_site: SourceSpan,
    ) -> Type {
        let name = "Fn(".to_string()
            + &parameter_types
                .iter()
                .map(|ty| ty.to_string())
                .collect::<Vec<String>>()
                .join(", ")
            + ") -> "
            + &return_type.to_string();
        Type::function(&name, parameter_types, return_type, declaration_site)
    }

    pub fn make_struct(
        &self,
        name: &Token,
        fields: HashMap<SharedString, StructField>,
        declaration_site: SourceSpan,
    ) -> Type {
        Type::new(
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
        name: &Token,
        //        symbol_map: HashMap<SharedString, InterpreterValue>,
        declaration_site: SourceSpan,
    ) -> Type {
        Type::new(name.lexeme(), TypeKind::Namespace, declaration_site)
    }

    pub fn make_trait(&self, name: &Token, declaration_site: SourceSpan) -> Type {
        Type::new(
            name.lexeme(),
            TypeKind::Trait(TraitType { name: name.clone() }),
            declaration_site,
        )
    }
}
