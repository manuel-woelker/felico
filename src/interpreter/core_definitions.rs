use std::rc::Rc;
use crate::frontend::ast::types::{PrimitiveType, Type, TypeKind};
use crate::infra::result::bail;
use crate::infra::shared_string::SharedString;
use crate::interpreter::value::{InterpreterValue, ValueFactory, ValueKind};

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

factory_fns!(bool, unit, f64, function, ty, string, unknown);


impl TypeFactory {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(TypeFactoryInner {
                bool: Type::primitive("bool", PrimitiveType::Bool),
                unit: Type::primitive("()", PrimitiveType::Unit),
                f64: Type::primitive("f64", PrimitiveType::F64),
                string: Type::primitive("string", PrimitiveType::String),
                function: Type::primitive("FUNCTION", PrimitiveType::Unit),
                ty: Type::primitive("Type", PrimitiveType::Unit),
                unknown: Type::new("unknown", TypeKind::Unknown),
            }),
        }
    }

}

pub fn get_core_definitions(type_factory: &TypeFactory) -> Vec<CoreDefinition> {
    let mut core_definitions = Vec::new();
    let mut add_definition = |name: &str, value: InterpreterValue| {
        core_definitions.push(CoreDefinition {
            name: SharedString::from(name),
            value: value.into(),
        });
    };
    let value_factory = ValueFactory::new(type_factory);

    add_definition("bool", value_factory.new_type(type_factory.bool()));
    let value_factory_clone = value_factory.clone();
    add_definition("sqrt", value_factory.new_native_callable("sqrt", 1, move |_interpreter, arguments| {
        if let ValueKind::Number(arg) = arguments[0].val {
            Ok(value_factory_clone.f64(arg.sqrt()))
        } else {
            bail!("Expected number as argument to sqrt")
        }
    }));
    let value_factory_clone = value_factory.clone();
    add_definition("debug_print", value_factory.new_native_callable("debug_print", 1, move |interpreter, arguments| {
        for argument in &arguments {
            interpreter.print(argument);
        }
        Ok(value_factory_clone.unit())
    }));
    core_definitions
}