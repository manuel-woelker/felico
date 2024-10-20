use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::qualified_name::QualifiedName;
use crate::infra::result::{bail, FelicoResult};
use crate::interpret::value::{InterpreterValue, ValueKind};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::sync::Mutex;

pub struct EnvironmentInner<'ws> {
    values: HashMap<String, InterpreterValue<'ws>>,
    parent: Option<Environment<'ws>>,
}

#[derive(Clone)]
pub struct Environment<'ws> {
    inner: Rc<Mutex<EnvironmentInner<'ws>>>,
}

impl<'ws> Debug for Environment<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Environment <{} entries>",
            self.inner.lock().unwrap().values.len()
        )
    }
}
impl<'ws> Default for Environment<'ws> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ws> Environment<'ws> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(Mutex::new(EnvironmentInner {
                values: Default::default(),
                parent: None,
            })),
        }
    }
    pub fn define(&self, name: &str, value: InterpreterValue<'ws>) {
        self.inner
            .lock()
            .unwrap()
            .values
            .insert(name.to_string(), value);
    }

    pub fn assign(&self, name: &str, value: InterpreterValue<'ws>) -> FelicoResult<()> {
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

    pub fn get(&self, name: &str) -> FelicoResult<InterpreterValue<'ws>> {
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
        qualified_name: &AstNode<'ws, QualifiedName<'ws>>,
        distance: i32,
    ) -> FelicoResult<InterpreterValue<'ws>> {
        let parts = &qualified_name.data.parts;
        let name = parts[0].lexeme();
        let environment = self.get_environment_at_distance(name, distance)?;
        let borrowed = environment.inner.lock().unwrap();
        let option = borrowed.values.get(name);
        if let Some(value) = option {
            let mut result = value.clone();
            for part in parts.iter().skip(1) {
                let ValueKind::SymbolMap(ref symbol_map) = result.val else {
                    bail!(
                        "When resolving {}: Not a symbol '{:?}'",
                        qualified_name,
                        result
                    );
                };
                let symbol_entry = symbol_map.get_symbol(part.lexeme())?;
                let Some(symbol_value) = symbol_entry else {
                    bail!(
                        "When resolving {}: Could not find '{:?}'",
                        qualified_name,
                        part.lexeme()
                    );
                };
                result = symbol_value;
            }
            Ok(result)
        } else {
            bail!(
                "No variable named '{}' defined (get at distance {}) ",
                name,
                distance
            );
        }
    }

    fn get_environment_at_distance(
        &self,
        name: &str,
        distance: i32,
    ) -> FelicoResult<Environment<'ws>> {
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
        value: InterpreterValue<'ws>,
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
