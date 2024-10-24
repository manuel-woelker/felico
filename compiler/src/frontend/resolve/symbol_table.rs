use crate::frontend::resolve::symbol::Symbol;
use crate::infra::shared_string::SharedString;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct SymbolTable<'ws> {
    inner: Mutex<SymbolTableInner<'ws>>,
}

#[derive(Debug)]
pub struct SymbolTableInner<'ws> {
    symbols: HashMap<SharedString<'ws>, Symbol<'ws>>,
}

impl<'ws> SymbolTable<'ws> {
    pub fn new() -> SymbolTable<'ws> {
        SymbolTable {
            inner: Mutex::new(SymbolTableInner {
                symbols: Default::default(),
            }),
        }
    }

    pub fn get(&self, name: SharedString) -> Option<Symbol<'ws>> {
        self.inner.lock().unwrap().symbols.get(name).copied()
    }

    pub(crate) fn insert(&self, name: SharedString<'ws>, symbol: Symbol<'ws>) {
        self.inner.lock().unwrap().symbols.insert(name, symbol);
    }
}

pub type SymbolMap<'ws> = HashMap<SharedString<'ws>, Symbol<'ws>>;
