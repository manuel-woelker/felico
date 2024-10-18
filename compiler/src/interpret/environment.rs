use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::qualified_name::QualifiedName;
use crate::infra::result::{bail, FelicoResult};
use crate::interpret::value::{InterpreterValue, ValueKind};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::sync::Mutex;

pub struct EnvironmentInner<'a> {
    values: HashMap<String, InterpreterValue<'a>>,
    parent: Option<Environment<'a>>,
}

#[derive(Clone)]
pub struct Environment<'a> {
    inner: Rc<Mutex<EnvironmentInner<'a>>>,
}

impl<'a> Debug for Environment<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Environment <{} entries>",
            self.inner.lock().unwrap().values.len()
        )
    }
}
impl<'a> Default for Environment<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Environment<'a> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(Mutex::new(EnvironmentInner {
                values: Default::default(),
                parent: None,
            })),
        }
    }
    pub fn define(&self, name: &str, value: InterpreterValue) {
        self.inner
            .lock()
            .unwrap()
            .values
            .insert(name.to_string(), value);
    }

    pub fn assign(&self, name: &str, value: InterpreterValue) -> FelicoResult<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(destination) = inner.values.get_mut(name) {
            *destination = value;
            return Ok(());
        }
        if let Some(parent) = &inner.parent {
            parent.assign(name, value)?;
            Ok(())
        } else {
            bail!("No variable named '{}' defined (assign)", name);
        }
    }

    pub fn get(&self, name: &str) -> FelicoResult<InterpreterValue> {
        let inner = self.inner.lock().unwrap();
        if let Some(value) = inner.values.get(name) {
            return Ok(value.clone());
        }
        if let Some(parent) = &inner.parent {
            parent.get(name)
        } else {
            bail!("No variable named '{}' defined (get)", name);
        }
    }

    pub fn get_at_distance(
        &self,
        qualified_name: &AstNode<QualifiedName>,
        distance: i32,
    ) -> FelicoResult<InterpreterValue> {
        let parts = &qualified_name.data.parts;
        let name = parts[0].lexeme();
        let environment = self.get_environment_at_distance(name, distance)?;
        let borrowed = environment.inner.lock().unwrap();
        if let Some(value) = borrowed.values.get(name) {
            let mut value = value.clone();
            if parts.len() <= 1 {
                return Ok(value.clone());
            }
            for part in parts.iter().skip(1) {
                let ValueKind::SymbolMap(symbol_map) = &value.val else {
                    bail!(
                        "When resolving {}: Not a symbol '{:?}'",
                        qualified_name,
                        value
                    );
                };
                let Some(symbol_value) = &symbol_map.get_symbol(part.lexeme())? else {
                    bail!(
                        "When resolving {}: Could not find '{:?}'",
                        qualified_name,
                        part.lexeme()
                    );
                };
                value = symbol_value.clone();
            }
            Ok(value)
        } else {
            bail!(
                "No variable named '{}' defined (get at distance {}) ",
                name,
                distance
            );
        }
    }

    fn get_environment_at_distance(&self, name: &str, distance: i32) -> FelicoResult<Environment> {
        let mut environment = self.clone();
        for _ in 0..distance {
            let cloned = environment.clone();
            let borrowed = cloned.inner.lock().unwrap();
            if let Some(parent) = &borrowed.parent {
                environment = parent.clone();
            } else {
                bail!("Failed to get parent when retrieving '{}'", name);
            }
        }
        Ok(environment)
    }

    pub(crate) fn assign_at_distance(
        &self,
        qualified_name: &AstNode<QualifiedName>,
        distance: i32,
        value: InterpreterValue,
    ) -> FelicoResult<()> {
        let name = qualified_name.data.parts[0].lexeme();
        let environment = self.get_environment_at_distance(name, distance)?;
        let mut borrowed = environment.inner.lock().unwrap();
        if let Some(slot) = borrowed.values.get_mut(name) {
            *slot = value;
            Ok(())
        } else {
            bail!(
                "No variable named '{}' defined (get at distance {}) ",
                qualified_name,
                distance
            );
        }
    }

    pub fn child_environment(&self) -> Self {
        Self {
            inner: Rc::new(Mutex::new(EnvironmentInner {
                values: Default::default(),
                parent: Some(self.clone()),
            })),
        }
    }

    pub fn enter_new(&mut self) {
        *self = self.child_environment()
    }

    pub fn exit(&mut self) {
        let inner = self.inner.lock().unwrap();
        let parent = inner.parent.clone();
        std::mem::drop(inner);
        if let Some(parent) = &parent {
            *self = parent.clone();
        } else {
            panic!("No parent environment")
        }
    }
}
