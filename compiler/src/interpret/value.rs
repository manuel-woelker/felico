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
pub struct InterpreterValue<'a> {
    pub val: ValueKind<'a>,
    pub ty: Type<'a>,
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

impl<'a> ValueFactory<'a> {
    pub fn new(type_factory: TypeFactory<'a>, arena: &'a Arena) -> Self {
        let inner = arena.alloc(ValueFactoryInner { type_factory });
        Self { inner }
    }

    pub fn f64(&self, value: f64) -> InterpreterValue<'a> {
        InterpreterValue {
            val: ValueKind::F64(value),
            ty: self.inner.type_factory.f64(),
        }
    }

    pub fn i64(&self, value: i64) -> InterpreterValue<'a> {
        InterpreterValue {
            val: ValueKind::I64(value),
            ty: self.inner.type_factory.i64(),
        }
    }

    pub fn bool(&self, value: bool) -> InterpreterValue<'a> {
        InterpreterValue {
            val: ValueKind::Bool(value),
            ty: self.inner.type_factory.bool(),
        }
    }
    pub fn unit(&self) -> InterpreterValue<'a> {
        InterpreterValue {
            val: ValueKind::Unit,
            ty: self.inner.type_factory.unit(),
        }
    }
    pub fn callable(&self, callable: Callable<'a>, ty: Type<'a>) -> InterpreterValue<'a> {
        InterpreterValue {
            val: ValueKind::Callable(callable),
            ty,
        }
    }

    pub fn new_type(&self, ty: Type<'a>) -> InterpreterValue<'a> {
        InterpreterValue {
            val: ValueKind::Type(ty),
            ty: self.inner.type_factory.ty(),
        }
    }

    pub fn new_string(&self, s: String) -> InterpreterValue<'a> {
        InterpreterValue {
            val: ValueKind::String(s),
            ty: self.inner.type_factory.str(),
        }
    }

    pub fn panic(&self, message: String, stack: Vec<StackFrame>) -> InterpreterValue<'a> {
        todo!("implement");
        /*
        InterpreterValue {
            val: ValueKind::Panic(Rc::new(Panic { message, stack })),
            ty: self.type_factory.unit(),
        }*/
    }

    pub fn new_native_callable(
        &self,
        name: &str,
        arity: usize,
        fun: impl Fn(
                &mut Interpreter<'a>,
                Vec<InterpreterValue<'a>>,
            ) -> FelicoResult<InterpreterValue<'a>>
            + 'a,
        ty: Type<'a>,
    ) -> InterpreterValue<'a> {
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
        ty: &Type<'a>,
        fields: HashMap<SharedString<'a>, InterpreterValue<'a>>,
    ) -> InterpreterValue<'a> {
        InterpreterValue {
            val: ValueKind::StructInstance(StructInstance::new(fields)),
            ty: *ty,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueKind<'a> {
    Unit,
    Return(Box<InterpreterValue<'a>>),
    Panic(&'a Panic<'a>),
    Tuple(Vec<InterpreterValue<'a>>),
    String(String),
    Bool(bool),
    F64(f64),
    I64(i64),
    Callable(Callable<'a>),
    Type(Type<'a>),
    StructInstance(StructInstance<'a>),
    SymbolMap(ValueMap<'a>),
}

impl<'a> Display for InterpreterValue<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.val, f)
    }
}

impl<'a> Debug for InterpreterValue<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.val, f)
    }
}

impl<'a> Display for ValueKind<'a> {
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
pub struct Callable<'a> {
    pub name: String,
    pub arity: usize,
    pub fun: Rc<CallableFun<'a>>,
}

pub type NativeFunction<'a> = Box<
    dyn Fn(&'a mut Interpreter<'a>, Vec<InterpreterValue<'a>>) -> FelicoResult<InterpreterValue<'a>>
        + 'a,
>;

pub enum CallableFun<'a> {
    Native(NativeFunction<'a>),
    Defined(DefinedFunction<'a>),
}

pub struct DefinedFunction<'a> {
    pub fun_stmt: FunStmt<'a>,
    pub closure: Environment<'a>,
}

impl<'a> Debug for Callable<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Callable '{}/{}'", self.name, self.arity)
    }
}

impl<'ws> Display for Panic<'ws> {
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
pub struct Panic<'a> {
    pub message: String,
    pub stack: Vec<StackFrame<'a>>,
}

#[derive(Debug, Clone)]
pub struct StructInstance<'a> {
    pub inner: Rc<RefCell<StructInstanceInner<'a>>>,
}

impl<'a> StructInstance<'a> {
    pub fn new(fields: HashMap<SharedString<'a>, InterpreterValue<'a>>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StructInstanceInner { fields })),
        }
    }

    pub fn set_field(
        &self,
        field_name: SharedString<'a>,
        value: InterpreterValue<'a>,
    ) -> FelicoResult<()> {
        self.inner.borrow_mut().fields.insert(field_name, value);
        Ok(())
    }
    pub fn get_field(&self, field_name: &str) -> FelicoResult<Option<InterpreterValue<'a>>> {
        let inner = self.inner.borrow();
        let Some(value) = inner.fields.get(field_name) else {
            return Ok(None);
        };
        Ok(Some(value.clone()))
    }
}

#[derive(Debug, Clone)]
pub struct StructInstanceInner<'a> {
    pub fields: HashMap<SharedString<'a>, InterpreterValue<'a>>,
}

#[derive(Debug, Clone)]
pub struct ValueMap<'a> {
    pub inner: Rc<RefCell<ValueMapInner<'a>>>,
}

impl<'a> ValueMap<'a> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ValueMapInner {
                symbols: HashMap::new(),
            })),
        }
    }

    pub fn set_symbol(
        &self,
        field_name: SharedString<'a>,
        value: InterpreterValue<'a>,
    ) -> FelicoResult<()> {
        self.inner.borrow_mut().symbols.insert(field_name, value);
        Ok(())
    }
    pub fn get_symbol(&self, field_name: &str) -> FelicoResult<Option<InterpreterValue<'a>>> {
        let inner = self.inner.borrow();
        let Some(value) = inner.symbols.get(field_name) else {
            return Ok(None);
        };
        Ok(Some(value.clone()))
    }
}

#[derive(Debug)]
pub struct ValueMapInner<'a> {
    pub symbols: HashMap<SharedString<'a>, InterpreterValue<'a>>,
}

pub trait Namespace<'a> {
    fn resolve(&self, name: &str) -> FelicoResult<Option<InterpreterValue<'a>>>;
}

impl<'a> Namespace<'a> for ValueMap<'a> {
    fn resolve(&self, name: &str) -> FelicoResult<Option<InterpreterValue<'a>>> {
        self.get_symbol(name)
    }
}
