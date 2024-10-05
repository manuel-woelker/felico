use once_cell::sync::Lazy;
use crate::infra::result::bail;
use crate::infra::shared_string::SharedString;
use crate::interpreter::value::{create_native_callable, InterpreterValue, PrimitiveType, Type, TypeKind, ValueKind};

pub struct CoreDefinition {
    pub name: SharedString,
    pub value: InterpreterValue,
}


pub static TYPE_BOOL: Lazy<Type> = Lazy::new(|| {
    Type::primitive("bool", PrimitiveType::Bool)
});
pub static TYPE_F64: Lazy<Type> = Lazy::new(|| {
    Type::primitive("f64", PrimitiveType::F64)
});

pub static TYPE_UNIT: Lazy<Type> = Lazy::new(|| {
    Type::primitive("()", PrimitiveType::Unit)
});
pub static TYPE_FUNCTION: Lazy<Type> = Lazy::new(|| {
    Type::primitive("FUNCTION", PrimitiveType::Unit)
});
pub static TYPE_TYPE: Lazy<Type> = Lazy::new(|| {
    Type::primitive("Type", PrimitiveType::Unit)
});
pub static TYPE_STRING: Lazy<Type> = Lazy::new(|| {
    Type::primitive("String", PrimitiveType::String)
});
pub static TYPE_UNKNOWN: Lazy<Type> = Lazy::new(|| {
    Type::new("unknown", TypeKind::Unknown)
});


static CORE_DEFINITIONS: Lazy<Vec<CoreDefinition>> = Lazy::new(|| {
    let mut core_definitions = Vec::new();
    let mut add_definition = |name: &str, value: InterpreterValue| {
        core_definitions.push(CoreDefinition {
            name: SharedString::from(name),
            value: value.into(),
        });
    };

    add_definition("bool", TYPE_BOOL.clone().into());

    add_definition("sqrt", create_native_callable("sqrt", 1,|_interpreter, arguments| {
        if let ValueKind::Number(arg) = arguments[0].val {
            Ok(InterpreterValue::f64(arg.sqrt()))
        } else {
            bail!("Expected number as argument to sqrt")
        }
    }));
    add_definition("debug_print", create_native_callable("debug_print", 1,|interpreter, arguments| {
        for argument in &arguments {
            (interpreter.print_fn)(argument);
        }
        Ok(InterpreterValue::unit())
    }));
    core_definitions
});

pub fn get_core_definitions() -> &'static [CoreDefinition] {
    &CORE_DEFINITIONS
}