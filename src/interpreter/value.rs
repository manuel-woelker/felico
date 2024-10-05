use crate::frontend::ast::stmt::FunStmt;
use crate::infra::result::FelicoResult;
use crate::interpreter::environment::Environment;
use crate::interpreter::interpreter::Interpreter;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use crate::infra::shared_string::SharedString;
use crate::interpreter::core_definitions::{TYPE_BOOL, TYPE_F64, TYPE_FUNCTION, TYPE_TYPE};

#[derive(Clone)]
pub struct InterpreterValue {
    pub val: ValueKind,
    pub ty: Type,
}


impl InterpreterValue {
    pub fn f64(value: f64) -> Self {
        Self {
            val: ValueKind::Number(value),
            ty: TYPE_F64.clone(),
        }
    }
    pub fn bool(value: bool) -> Self {
        Self {
            val: ValueKind::Bool(value),
            ty: TYPE_BOOL.clone(),
        }
    }
    pub fn unit() -> Self {
        Self {
            val: ValueKind::Unit,
            ty: TYPE_BOOL.clone(),
        }
    }
    pub fn callable(callable: Callable) -> Self {
        Self {
            val: ValueKind::Callable(callable),
            ty: TYPE_FUNCTION.clone(),
        }
    }

    pub fn new_type(ty: Type) -> Self {
        Self {
            val: ValueKind::Type(ty),
            ty: TYPE_TYPE.clone(),
        }
    }

    pub fn new_string(s: String) -> Self {
        Self {
            val: ValueKind::String(s),
            ty: TYPE_TYPE.clone(),
        }
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
    InterpreterValue::callable(Callable {
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

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "〈{}〉", self.inner.name)
    }
}

impl Type {
    pub fn new<S: Into<SharedString>>(name: S, kind: TypeKind) -> Self {
        Self {
            inner: Arc::new(TypeInner {
                name: name.into(),
                kind,
            }),
        }
    }

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

impl PartialEq for Type {
    fn eq(&self, other: &Type) -> bool {
        self.inner.kind == other.inner.kind
    }
}


#[derive(Debug)]
pub struct TypeInner {
    name: SharedString,
    kind: TypeKind,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TypeKind {
    Unknown,
    Primitive(PrimitiveType),
}

#[derive(Debug, Eq, PartialEq)]
pub enum PrimitiveType {
    Bool,
    Unit,
    F64,
    String,
}

impl From<Type> for InterpreterValue {
    fn from(value: Type) -> Self {
        InterpreterValue::new_type(value)
    }
}