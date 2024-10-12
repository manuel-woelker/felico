use crate::frontend::ast::stmt::FunStmt;
use crate::frontend::ast::types::Type;
use crate::infra::result::FelicoResult;
use crate::infra::source_span::SourceSpan;
use crate::interpret::core_definitions::TypeFactory;
use crate::interpret::environment::Environment;
use crate::interpret::interpreter::Interpreter;
use itertools::{Itertools, Position};
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

#[derive(Clone)]
pub struct InterpreterValue {
    pub val: ValueKind,
    pub ty: Type,
}

impl InterpreterValue {
    pub fn with_panic_stack_frame(&mut self, location: &SourceSpan) {
        let ValueKind::Panic(panic) = &self.val else {
            return;
        };
        let mut stack = panic.stack.clone();
        stack.push(location.clone());
        *self = Self {
            val: ValueKind::Panic(Rc::new(Panic {
                message: panic.message.clone(),
                stack,
            })),
            ty: self.ty.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ValueFactory {
    type_factory: TypeFactory,
}

impl ValueFactory {}

impl ValueFactory {
    pub fn new(type_factory: &TypeFactory) -> Self {
        Self {
            type_factory: type_factory.clone(),
        }
    }

    pub fn f64(&self, value: f64) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::F64(value),
            ty: self.type_factory.f64(),
        }
    }

    pub fn i64(&self, value: i64) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::I64(value),
            ty: self.type_factory.i64(),
        }
    }

    pub fn bool(&self, value: bool) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Bool(value),
            ty: self.type_factory.bool(),
        }
    }
    pub fn unit(&self) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Unit,
            ty: self.type_factory.unit(),
        }
    }
    pub fn callable(&self, callable: Callable, ty: Type) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Callable(callable),
            ty,
        }
    }

    pub fn new_type(&self, ty: Type) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Type(ty),
            ty: self.type_factory.ty(),
        }
    }

    pub fn new_string(&self, s: String) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::String(s),
            ty: self.type_factory.str(),
        }
    }

    pub fn panic(&self, message: String) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Panic(Rc::new(Panic {
                message,
                stack: vec![],
            })),
            ty: self.type_factory.unit(),
        }
    }

    pub fn new_native_callable(
        &self,
        name: &str,
        arity: usize,
        fun: impl Fn(&mut Interpreter, Vec<InterpreterValue>) -> FelicoResult<InterpreterValue>
            + 'static,
        ty: Type,
    ) -> InterpreterValue {
        self.callable(
            Callable {
                name: name.to_string(),
                arity,
                fun: Rc::new(CallableFun::Native(Box::new(fun))),
            },
            ty,
        )
    }
}

#[derive(Debug, Clone)]
pub enum ValueKind {
    Unit,
    Return(Box<InterpreterValue>),
    Panic(Rc<Panic>),
    Tuple(Vec<InterpreterValue>),
    String(String),
    Bool(bool),
    F64(f64),
    I64(i64),
    Callable(Callable),
    Type(Type),
}

impl Display for InterpreterValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.val, f)
    }
}

impl Debug for InterpreterValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.val, f)
    }
}

impl Display for ValueKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueKind::Unit => f.write_str("()"),
            ValueKind::Tuple(tuple) => {
                f.write_str("(")?;
                for (pos, component) in tuple.iter().with_position() {
                    Display::fmt(component, f)?;
                    if pos != Position::Last && pos != Position::Only {
                        f.write_str(", ")?;
                    }
                }
                f.write_str(")")
            }
            ValueKind::String(s) => f.write_str(s),
            ValueKind::Bool(bool) => {
                if *bool {
                    f.write_str("true")
                } else {
                    f.write_str("false")
                }
            }
            ValueKind::F64(number) => {
                write!(f, "{}", number)
            }
            ValueKind::I64(number) => {
                write!(f, "{}", number)
            }
            ValueKind::Callable(callable) => {
                write!(f, "{}/{}", callable.name, callable.arity)
            }
            ValueKind::Type(ty) => {
                write!(f, "{:?}", ty)
            }
            ValueKind::Return(value) => {
                write!(f, "ret {:?}", value)
            }
            ValueKind::Panic(message) => {
                write!(f, "panic {:?}", message)
            }
        }
    }
}

#[derive(Clone)]
pub struct Callable {
    pub name: String,
    pub arity: usize,
    pub fun: Rc<CallableFun>,
}

pub type NativeFunction =
    Box<dyn Fn(&mut Interpreter, Vec<InterpreterValue>) -> FelicoResult<InterpreterValue>>;

pub enum CallableFun {
    Native(NativeFunction),
    Defined(DefinedFunction),
}

pub struct DefinedFunction {
    pub fun_stmt: FunStmt,
    pub closure: Environment,
}

impl Debug for Callable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Callable '{}/{}'", self.name, self.arity)
    }
}

#[derive(Debug, Clone)]
pub struct Panic {
    pub message: String,
    pub stack: Vec<SourceSpan>,
}

impl Display for Panic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.message, f)?;
        for location in &self.stack {
            write!(
                f,
                "\n\t[{}:{}:{}] {}",
                location.source_file.filename(),
                location.get_line_number(),
                location.get_column_number(),
                location.get_source_code(),
            )?;
        }
        Ok(())
    }
}
