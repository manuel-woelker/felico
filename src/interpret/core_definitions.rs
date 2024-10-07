use crate::frontend::ast::types::{PrimitiveType, StructField, StructType, Type, TypeKind};
use crate::frontend::lex::token::Token;
use crate::infra::result::bail;
use crate::infra::shared_string::SharedString;
use crate::interpret::value::{InterpreterValue, ValueFactory, ValueKind};
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

factory_fns!(bool, unit, i64, f64, ty, str, unknown);

impl TypeFactory {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(TypeFactoryInner {
                bool: Type::primitive("bool", PrimitiveType::Bool),
                unit: Type::tuple("()", vec![]),
                f64: Type::primitive("f64", PrimitiveType::F64),
                i64: Type::primitive("i64", PrimitiveType::I64),
                str: Type::primitive("str", PrimitiveType::Str),
                ty: Type::ty(),
                unknown: Type::new("unknown", TypeKind::Unknown),
            }),
        }
    }

    pub fn function(&self, parameter_types: Vec<Type>, return_type: Type) -> Type {
        let name = "Fn(".to_string()
            + &parameter_types
                .iter()
                .map(|ty| ty.to_string())
                .collect::<Vec<String>>()
                .join(", ")
            + ") -> "
            + &return_type.to_string();
        Type::function(&name, parameter_types, return_type)
    }

    pub fn tuple(&self, components: Vec<Type>) -> Type {
        let name = "(".to_string()
            + &components
                .iter()
                .map(|ty| ty.to_string())
                .collect::<Vec<String>>()
                .join(", ")
            + ")";
        Type::tuple(&name, components)
    }

    pub fn make_struct(&self, name: &Token, fields: HashMap<SharedString, StructField>) -> Type {
        Type::new(
            name.lexeme(),
            TypeKind::Struct(StructType {
                name: name.clone(),
                fields,
            }),
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
            type_factory.function(vec![type_factory.f64()], type_factory.f64()),
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
                vec![Type::new("any".to_string(), TypeKind::Any)],
                type_factory.unit(),
            ),
        ),
    );
    core_definitions
}
