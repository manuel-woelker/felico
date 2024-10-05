use crate::frontend::ast::stmt::FunStmt;
use crate::infra::result::FelicoResult;
use crate::interpreter::environment::Environment;
use crate::interpreter::interpreter::Interpreter;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use crate::infra::shared_string::SharedString;

#[derive(Debug, Clone)]
pub enum InterpreterValue {
    Unit,
    String(String),
    Bool(bool),
    Number(f64),
    Callable(Callable),
    Type(Type),
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
            InterpreterValue::Type(ty) => {
                write!(f, "{:?}", ty)
            }
        }
    }
}


#[derive(Clone)]
pub struct Callable {
    pub name: String,
    pub arity: usize,
    pub fun: Arc<CallableFun>,
}

pub enum CallableFun {
    Native(Box<dyn Fn(&mut Interpreter, Vec<InterpreterValue>) -> FelicoResult<InterpreterValue> + Send + Sync>),
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

pub fn create_native_callable(name: &str, arity: usize, fun: impl Fn(&mut Interpreter, Vec<InterpreterValue>) -> FelicoResult<InterpreterValue> + Send + Sync + 'static) -> InterpreterValue {
    InterpreterValue::Callable(Callable {
        name: name.to_string(),
        arity,
        fun: Arc::new(CallableFun::Native(Box::new(fun))),
    })
}

#[derive(Clone)]
pub struct Type {
    inner: Arc<TypeInner>,
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "〈{}〉", self.inner.name)
    }
}

impl Type {
    pub fn primitive(name: &str, primitive_type: PrimitiveType) -> Self {
        Self {
            inner: Arc::new(TypeInner {
                name: SharedString::from(name),
                kind: TypeKind::Primitive(primitive_type),
            }),
        }
    }

    pub fn name(&self) -> &SharedString {
        &self.inner.name
    }
}

#[derive(Debug)]
pub struct TypeInner {
    name: SharedString,
    kind: TypeKind,
}

#[derive(Debug)]
pub enum TypeKind {
    Unknown,
    Primitive(PrimitiveType),
}

#[derive(Debug)]
pub enum PrimitiveType {
    Bool,
    F64,
}

impl From<Type> for InterpreterValue {
    fn from(value: Type) -> Self {
        InterpreterValue::Type(value)
    }
}