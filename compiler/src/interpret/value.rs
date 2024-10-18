use crate::frontend::ast::stmt::FunStmt;
use crate::infra::result::FelicoResult;
use crate::infra::shared_string::SharedString;
use crate::interpret::environment::Environment;
use crate::interpret::interpreter::{Interpreter, StackFrame};
use crate::model::type_factory::TypeFactory;
use crate::model::types::Type;
use itertools::{Itertools, Position};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

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

    pub fn panic(&self, message: String, stack: Vec<StackFrame>) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Panic(Rc::new(Panic { message, stack })),
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

    pub fn make_struct(
        &self,
        ty: &Type,
        fields: HashMap<SharedString, InterpreterValue>,
    ) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::StructInstance(StructInstance::new(fields)),
            ty: ty.clone(),
        }
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
    StructInstance(StructInstance),
    SymbolMap(ValueMap),
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
            ValueKind::StructInstance(struct_instance) => {
                write!(f, "Struct {:?}", struct_instance)
            }
            ValueKind::SymbolMap(symbol_map) => {
                write!(f, "SymbolMap {:?}", symbol_map)
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

impl Display for Panic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.message, f)?;
        for frame in self.stack.iter().rev() {
            let source_span = &frame.call_source_span;
            write!(
                f,
                "\n\t[{}:{}:{}] {}",
                source_span.source_file.filename(),
                source_span.get_line_number(),
                source_span.get_column_number(),
                source_span.get_source_code(),
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Panic {
    pub message: String,
    pub stack: Vec<StackFrame>,
}

#[derive(Debug, Clone)]
pub struct StructInstance {
    pub inner: Rc<RefCell<StructInstanceInner>>,
}

impl StructInstance {
    pub fn new(fields: HashMap<SharedString, InterpreterValue>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StructInstanceInner { fields })),
        }
    }

    pub fn set_field(&self, field_name: &str, value: InterpreterValue) -> FelicoResult<()> {
        self.inner
            .borrow_mut()
            .fields
            .insert(SharedString::from(field_name), value);
        Ok(())
    }
    pub fn get_field(&self, field_name: &str) -> FelicoResult<Option<InterpreterValue>> {
        let inner = self.inner.borrow();
        let Some(value) = inner.fields.get(field_name) else {
            return Ok(None);
        };
        Ok(Some(value.clone()))
    }
}

#[derive(Debug, Clone)]
pub struct StructInstanceInner {
    pub fields: HashMap<SharedString, InterpreterValue>,
}

#[derive(Debug, Clone)]
pub struct ValueMap {
    pub inner: Rc<RefCell<ValueMapInner>>,
}

impl ValueMap {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ValueMapInner {
                symbols: HashMap::new(),
            })),
        }
    }

    pub fn set_symbol(&self, field_name: &str, value: InterpreterValue) -> FelicoResult<()> {
        self.inner
            .borrow_mut()
            .symbols
            .insert(SharedString::from(field_name), value);
        Ok(())
    }
    pub fn get_symbol(&self, field_name: &str) -> FelicoResult<Option<InterpreterValue>> {
        let inner = self.inner.borrow();
        let Some(value) = inner.symbols.get(field_name) else {
            return Ok(None);
        };
        Ok(Some(value.clone()))
    }
}

#[derive(Debug)]
pub struct ValueMapInner {
    pub symbols: HashMap<SharedString, InterpreterValue>,
}

pub trait Namespace {
    fn resolve(&self, name: &str) -> FelicoResult<Option<InterpreterValue>>;
}

impl Namespace for ValueMap {
    fn resolve(&self, name: &str) -> FelicoResult<Option<InterpreterValue>> {
        self.get_symbol(name)
    }
}
