use crate::frontend::ast::stmt::FunStmt;
use crate::infra::result::FelicoResult;
use crate::interpreter::environment::Environment;
use crate::interpreter::interpreter::Interpreter;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use crate::frontend::ast::types::Type;
use crate::interpreter::core_definitions::TypeFactory;

#[derive(Clone)]
pub struct InterpreterValue {
    pub val: ValueKind,
    pub ty: Type,
}

#[derive(Clone)]
pub struct ValueFactory {
    type_factory: TypeFactory,
}

impl ValueFactory {
    pub fn new(type_factory: &TypeFactory) -> Self {
        Self {
            type_factory: type_factory.clone(),
        }
    }

    pub fn f64(&self, value: f64) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Number(value),
            ty: self.type_factory.f64(),
        }
    }
    pub fn bool(&self, value: bool) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Bool(value),
            ty: self.type_factory.bool(),
        }
    }
    pub fn unit(&self, ) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Unit,
            ty: self.type_factory.unit(),
        }
    }
    pub fn callable(&self, callable: Callable) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Callable(callable),
            ty: self.type_factory.function(),
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
            ty: self.type_factory.string(),
        }
    }


    pub fn new_native_callable(&self, name: &str, arity: usize, fun: impl Fn(&mut Interpreter, Vec<InterpreterValue>) -> FelicoResult<InterpreterValue> + 'static) -> InterpreterValue {
        self.callable(Callable {
            name: name.to_string(),
            arity,
            fun: Rc::new(CallableFun::Native(Box::new(fun))),
        })
    }

}

#[derive(Debug, Clone)]
pub enum ValueKind {
    Unit,
    String(String),
    Bool(bool),
    Number(f64),
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
            ValueKind::Unit => {
                f.write_str("()")
            }
            ValueKind::String(s) => {
                f.write_str(s)
            }
            ValueKind::Bool(bool) => {
                if *bool {
                    f.write_str("true")
                } else {
                    f.write_str("false")
                }
            }
            ValueKind::Number(number) => {
                write!(f, "{}", number)
            }
            ValueKind::Callable(callable) => {
                write!(f, "{}/{}", callable.name, callable.arity)
            }
            ValueKind::Type(ty) => {
                write!(f, "{:?}", ty)
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

pub enum CallableFun {
    Native(Box<dyn Fn(&mut Interpreter, Vec<InterpreterValue>) -> FelicoResult<InterpreterValue>>),
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

