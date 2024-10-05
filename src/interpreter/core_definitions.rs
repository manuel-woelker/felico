use once_cell::sync::Lazy;
use crate::infra::shared_string::SharedString;

pub struct CoreDefinition {
    pub name: SharedString,
//    value: InterpreterValue,
}

static CORE_DEFINITIONS: Lazy<Vec<CoreDefinition>> = Lazy::new(|| {
    let mut core_definitions = Vec::new();
    let mut add_definition = |name: &str| {
        core_definitions.push(CoreDefinition {
            name: SharedString::from(name),
            //        value: InterpreterValue::Unit,
        });
    };
    add_definition("sqrt");
    add_definition("debug_print");
    core_definitions
});

pub fn get_core_definitions() -> &'static [CoreDefinition] {
    &CORE_DEFINITIONS
}