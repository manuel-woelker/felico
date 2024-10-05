use crate::frontend::ast::stmt::FunStmt;
use crate::infra::result::FelicoResult;
use crate::interpreter::environment::Environment;
use crate::interpreter::interpreter::Interpreter;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum InterpreterValue {
    Unit,
    String(String),
    Bool(bool),
    Number(f64),
    Callable(Callable),
}

impl Display for InterpreterValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterValue::Unit => {
                f.write_str("()")
            }
            InterpreterValue::String(s) => {
                f.write_str(s)
            }
            InterpreterValue::Bool(bool) => {
                if *bool {
                    f.write_str("true")
                } else {
                    f.write_str("false")
                }
            }
            InterpreterValue::Number(number) => {
                write!(f, "{}", number)
            }
            InterpreterValue::Callable(callable) => {
                write!(f, "{}/{}", callable.name, callable.arity)
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

pub fn create_native_callable(name: &str, arity: usize, fun: impl Fn(&mut Interpreter, Vec<InterpreterValue>) -> FelicoResult<InterpreterValue> + 'static) -> InterpreterValue {
    InterpreterValue::Callable(Callable {
        name: name.to_string(),
        arity,
        fun: Rc::new(CallableFun::Native(Box::new(fun))),
    })
}
