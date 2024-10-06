use crate::infra::result::{bail, FelicoResult};
use crate::interpret::value::InterpreterValue;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::sync::Mutex;

pub struct EnvironmentInner {
    values: HashMap<String, InterpreterValue>,
    parent: Option<Environment>,
}

#[derive(Clone)]
pub struct Environment {
    inner: Rc<Mutex<EnvironmentInner>>,
}

impl Debug for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Environment <{} entries>",
            self.inner.lock().unwrap().values.len()
        )
    }
}
impl Environment {
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

    pub(crate) fn get_at_distance(
        &self,
        name: &str,
        distance: i32,
    ) -> FelicoResult<InterpreterValue> {
        let environment = self.get_environment_at_distance(name, distance)?;
        let borrowed = environment.inner.lock().unwrap();
        if let Some(value) = borrowed.values.get(name) {
            Ok(value.clone())
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
        name: &str,
        distance: i32,
        value: InterpreterValue,
    ) -> FelicoResult<()> {
        let environment = self.get_environment_at_distance(name, distance)?;
        let mut borrowed = environment.inner.lock().unwrap();
        if let Some(slot) = borrowed.values.get_mut(name) {
            *slot = value;
            Ok(())
        } else {
            bail!(
                "No variable named '{}' defined (get at distance {}) ",
                name,
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
