use crate::frontend::ast::types::{
    PrimitiveType, StructField, StructType, TraitType, Type, TypeKind,
};
use crate::frontend::lex::token::{Token, TokenType};
use crate::infra::result::bail;
use crate::infra::shared_string::SharedString;
use crate::infra::source_span::SourceSpan;
use crate::interpret::value::{InterpreterValue, ValueFactory, ValueKind};
use itertools::Itertools;
use std::collections::HashMap;
use std::rc::Rc;

pub struct CoreDefinition {
    pub name: SharedString,
    pub value: InterpreterValue,
}

macro_rules! factory_fns {
        ($($id:ident),+) => {
    #[derive(Debug)]
struct TypeFactoryInner {
            $(
            $id: Type,
            )+
}

    #[derive(Clone, Debug)]
    pub struct TypeFactory {
        inner: Rc<TypeFactoryInner>,
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

impl Default for TypeFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeFactory {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(TypeFactoryInner {
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

pub fn get_core_definitions(type_factory: &TypeFactory) -> Vec<CoreDefinition> {
    let mut core_definitions = Vec::new();
    let mut add_definition = |name: &str, value: InterpreterValue| {
        core_definitions.push(CoreDefinition {
            name: SharedString::from(name),
            value,
        });
    };
    let value_factory = ValueFactory::new(type_factory);

    add_definition("bool", value_factory.new_type(type_factory.bool()));
    add_definition("i64", value_factory.new_type(type_factory.i64()));
    add_definition("f64", value_factory.new_type(type_factory.f64()));
    add_definition("str", value_factory.new_type(type_factory.str()));
    add_definition("unit", value_factory.new_type(type_factory.unit()));
    let value_factory_clone = value_factory.clone();
    add_definition(
        "sqrt",
        value_factory.new_native_callable(
            "sqrt",
            1,
            move |_interpreter, arguments| {
                if let ValueKind::F64(arg) = arguments[0].val {
                    Ok(value_factory_clone.f64(arg.sqrt()))
                } else {
                    bail!("Expected number as argument to sqrt")
                }
            },
            type_factory.function(
                vec![type_factory.f64()],
                type_factory.f64(),
                SourceSpan::ephemeral(),
            ),
        ),
    );
    let value_factory_clone = value_factory.clone();
    add_definition(
        "debug_print",
        value_factory.new_native_callable(
            "debug_print",
            1,
            move |interpreter, arguments| {
                for argument in &arguments {
                    interpreter.print(argument);
                }
                Ok(value_factory_clone.unit())
            },
            type_factory.function(
                vec![Type::new_ephemeral("any".to_string(), TypeKind::Any)],
                type_factory.unit(),
                SourceSpan::ephemeral(),
            ),
        ),
    );
    let value_factory_clone = value_factory.clone();
    add_definition(
        "panic",
        value_factory.new_native_callable(
            "panic",
            1,
            move |interpreter, arguments| {
                Ok(value_factory_clone.panic(
                    arguments.iter().map(|arg| arg.to_string()).join(","),
                    interpreter.get_current_call_stack(),
                ))
            },
            type_factory.function(
                vec![type_factory.str()],
                type_factory.never(),
                SourceSpan::ephemeral(),
            ),
        ),
    );
    let value_factory_clone = value_factory.clone();
    add_definition(
        "abs",
        value_factory.new_native_callable(
            "abs",
            1,
            move |_interpreter, arguments| {
                if let ValueKind::F64(arg) = arguments[0].val {
                    Ok(value_factory_clone.f64(arg.abs()))
                } else {
                    bail!("Expected number as argument to sqrt")
                }
            },
            type_factory.function(
                vec![type_factory.f64()],
                type_factory.f64(),
                SourceSpan::ephemeral(),
            ),
        ),
    );
    core_definitions
}
