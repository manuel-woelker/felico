use crate::infra::result::bail;
use crate::infra::shared_string::SharedString;
use crate::infra::source_span::SourceSpan;
use crate::interpret::value::{InterpreterValue, ValueFactory, ValueKind};
use crate::model::type_factory::TypeFactory;
use crate::model::types::TypeKind;
use itertools::Itertools;

pub struct CoreDefinition<'ws> {
    pub name: SharedString<'ws>,
    pub value: InterpreterValue<'ws>,
}

pub fn get_core_definitions<'ws>(
    value_factory: ValueFactory<'ws>,
    type_factory: TypeFactory<'ws>,
) -> Vec<CoreDefinition<'ws>> {
    let mut core_definitions = Vec::new();
    let mut add_definition = |name: SharedString<'ws>, value: InterpreterValue<'ws>| {
        core_definitions.push(CoreDefinition {
            name: SharedString::from(name),
            value,
        });
    };

    add_definition("bool", value_factory.new_type(type_factory.bool()));
    add_definition("i64", value_factory.new_type(type_factory.i64()));
    add_definition("f64", value_factory.new_type(type_factory.f64()));
    add_definition("str", value_factory.new_type(type_factory.str()));
    add_definition("unit", value_factory.new_type(type_factory.unit()));
    let value_factory_clone = value_factory;
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
                vec![type_factory.make_type("any", TypeKind::Any, SourceSpan::ephemeral())],
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
