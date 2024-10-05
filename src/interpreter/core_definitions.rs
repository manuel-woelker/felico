use once_cell::sync::Lazy;
use crate::infra::result::bail;
use crate::infra::shared_string::SharedString;
use crate::interpreter::value::{create_native_callable, InterpreterValue, PrimitiveType, Type};

pub struct CoreDefinition {
    pub name: SharedString,
    pub value: InterpreterValue,
}

static CORE_DEFINITIONS: Lazy<Vec<CoreDefinition>> = Lazy::new(|| {
    let mut core_definitions = Vec::new();
    let mut add_definition = |name: &str, value: InterpreterValue| {
        core_definitions.push(CoreDefinition {
            name: SharedString::from(name),
            value: value.into(),
        });
    };

    add_definition("bool", Type::primitive("bool", PrimitiveType::Bool).into());

    add_definition("sqrt", create_native_callable("sqrt", 1,|_interpreter, arguments| {
        if let InterpreterValue::Number(arg) = arguments[0] {
            Ok(InterpreterValue::Number(arg.sqrt()))
        } else {
            bail!("Expected number as argument to sqrt")
        }
    }));
    add_definition("debug_print", create_native_callable("debug_print", 1,|interpreter, arguments| {
        for argument in &arguments {
            (interpreter.print_fn)(argument);
        }
        Ok(InterpreterValue::Unit)
    }));
    core_definitions
});

pub fn get_core_definitions() -> &'static [CoreDefinition] {
    &CORE_DEFINITIONS
}