use crate::frontend::ast::stmt::FunStmt;
use crate::infra::arena::Arena;
use crate::infra::result::FelicoResult;
use crate::infra::shared_string::SharedString;
use crate::interpret::environment::Environment;
use crate::interpret::interpreter::Interpreter;
use crate::interpret::stack_frame::StackFrame;
use crate::model::type_factory::TypeFactory;
use crate::model::types::Type;
use itertools::{Itertools, Position};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

#[derive(Clone)]
pub struct InterpreterValue<'ws> {
    pub val: ValueKind<'ws>,
    pub ty: Type<'ws>,
}

#[derive(Clone)]
pub struct ValueFactory<'ws> {
    inner: &'ws ValueFactoryInner<'ws>,
}

impl<'ws> Copy for ValueFactory<'ws> {}

#[derive(Clone)]
pub struct ValueFactoryInner<'ws> {
    type_factory: TypeFactory<'ws>,
}

impl<'ws> ValueFactory<'ws> {
    pub fn new(type_factory: TypeFactory<'ws>, arena: &'ws Arena) -> Self {
        let inner = arena.alloc(ValueFactoryInner { type_factory });
        Self { inner }
    }

    pub fn f64(&self, value: f64) -> InterpreterValue<'ws> {
        InterpreterValue {
            val: ValueKind::F64(value),
            ty: self.inner.type_factory.f64(),
        }
    }

    pub fn i64(&self, value: i64) -> InterpreterValue<'ws> {
        InterpreterValue {
            val: ValueKind::I64(value),
            ty: self.inner.type_factory.i64(),
        }
    }

    pub fn bool(&self, value: bool) -> InterpreterValue<'ws> {
        InterpreterValue {
            val: ValueKind::Bool(value),
            ty: self.inner.type_factory.bool(),
        }
    }
    pub fn unit(&self) -> InterpreterValue<'ws> {
        InterpreterValue {
            val: ValueKind::Unit,
            ty: self.inner.type_factory.unit(),
        }
    }
    pub fn callable(&self, callable: Callable<'ws>, ty: Type<'ws>) -> InterpreterValue<'ws> {
        InterpreterValue {
            val: ValueKind::Callable(callable),
            ty,
        }
    }

    pub fn new_type(&self, ty: Type<'ws>) -> InterpreterValue<'ws> {
        InterpreterValue {
            val: ValueKind::Type(ty),
            ty: self.inner.type_factory.ty(),
        }
    }

    pub fn new_string(&self, s: String) -> InterpreterValue<'ws> {
        InterpreterValue {
            val: ValueKind::String(s),
            ty: self.inner.type_factory.str(),
        }
    }

    pub fn panic(&self, message: String, stack: Vec<StackFrame>) -> InterpreterValue<'ws> {
        let stack: Vec<String> = stack
            .iter()
            .map(|stack_frame: &StackFrame| {
                let source_span = &stack_frame.call_source_span;
                format!(
                    "[{}:{}:{}] {}",
                    source_span.source_file.filename(),
                    source_span.get_line_number(),
                    source_span.get_column_number(),
                    source_span.get_source_code(),
                )
            })
            .collect();
        InterpreterValue {
            val: ValueKind::Panic(Rc::new(Panic { message, stack })),
            ty: self.inner.type_factory.unit(),
        }
    }

    pub fn new_native_callable(
        &self,
        name: &str,
        arity: usize,
        fun: impl Fn(
                &mut Interpreter<'ws>,
                Vec<InterpreterValue<'ws>>,
            ) -> FelicoResult<InterpreterValue<'ws>>
            + 'ws,
        ty: Type<'ws>,
    ) -> InterpreterValue<'ws> {
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
        ty: Type<'ws>,
        fields: HashMap<SharedString<'ws>, InterpreterValue<'ws>>,
    ) -> InterpreterValue<'ws> {
        InterpreterValue {
            val: ValueKind::StructInstance(StructInstance::new(fields)),
            ty,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueKind<'ws> {
    Unit,
    Return(Box<InterpreterValue<'ws>>),
    Panic(Rc<Panic>),
    Tuple(Vec<InterpreterValue<'ws>>),
    String(String),
    Bool(bool),
    F64(f64),
    I64(i64),
    Callable(Callable<'ws>),
    Type(Type<'ws>),
    StructInstance(StructInstance<'ws>),
    SymbolMap(ValueMap<'ws>),
}

impl<'ws> Display for InterpreterValue<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.val, f)
    }
}

impl<'ws> Debug for InterpreterValue<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.val, f)
    }
}

impl<'ws> Display for ValueKind<'ws> {
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
pub struct Callable<'ws> {
    pub name: String,
    pub arity: usize,
    pub fun: Rc<CallableFun<'ws>>,
}

pub type NativeFunction<'ws> = Box<
    dyn Fn(&mut Interpreter<'ws>, Vec<InterpreterValue<'ws>>) -> FelicoResult<InterpreterValue<'ws>>
        + 'ws,
>;

pub enum CallableFun<'ws> {
    Native(NativeFunction<'ws>),
    Defined(DefinedFunction<'ws>),
}

pub struct DefinedFunction<'ws> {
    pub fun_stmt: FunStmt<'ws>,
    pub closure: Environment<'ws>,
}

impl<'ws> Debug for Callable<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Callable '{}/{}'", self.name, self.arity)
    }
}

impl Display for Panic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.message, f)?;
        for frame in self.stack.iter().rev() {
            f.write_str("\n    ")?;
            f.write_str(frame)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Panic {
    pub message: String,
    pub stack: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StructInstance<'ws> {
    pub inner: Rc<RefCell<StructInstanceInner<'ws>>>,
}

impl<'ws> StructInstance<'ws> {
    pub fn new(fields: HashMap<SharedString<'ws>, InterpreterValue<'ws>>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StructInstanceInner { fields })),
        }
    }

    pub fn set_field(
        &self,
        field_name: SharedString<'ws>,
        value: InterpreterValue<'ws>,
    ) -> FelicoResult<()> {
        self.inner.borrow_mut().fields.insert(field_name, value);
        Ok(())
    }
    pub fn get_field(&self, field_name: &str) -> FelicoResult<Option<InterpreterValue<'ws>>> {
        let inner = self.inner.borrow();
        let Some(value) = inner.fields.get(field_name) else {
            return Ok(None);
        };
        Ok(Some(value.clone()))
    }
}

#[derive(Debug, Clone)]
pub struct StructInstanceInner<'ws> {
    pub fields: HashMap<SharedString<'ws>, InterpreterValue<'ws>>,
}

#[derive(Debug, Clone)]
pub struct ValueMap<'ws> {
    pub inner: Rc<RefCell<ValueMapInner<'ws>>>,
}

impl<'ws> ValueMap<'ws> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ValueMapInner {
                symbols: HashMap::new(),
            })),
        }
    }

    pub fn set_symbol(
        &self,
        field_name: SharedString<'ws>,
        value: InterpreterValue<'ws>,
    ) -> FelicoResult<()> {
        self.inner.borrow_mut().symbols.insert(field_name, value);
        Ok(())
    }
    pub fn get_symbol(&self, field_name: &str) -> FelicoResult<Option<InterpreterValue<'ws>>> {
        let inner = self.inner.borrow();
        let Some(value) = inner.symbols.get(field_name) else {
            return Ok(None);
        };
        Ok(Some(value.clone()))
    }
}

#[derive(Debug)]
pub struct ValueMapInner<'ws> {
    pub symbols: HashMap<SharedString<'ws>, InterpreterValue<'ws>>,
}

pub trait Namespace<'ws> {
    fn resolve(&self, name: &str) -> FelicoResult<Option<InterpreterValue<'ws>>>;
}

impl<'ws> Namespace<'ws> for ValueMap<'ws> {
    fn resolve(&self, name: &str) -> FelicoResult<Option<InterpreterValue<'ws>>> {
        self.get_symbol(name)
    }
}
