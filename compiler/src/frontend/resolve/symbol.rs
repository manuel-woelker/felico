use crate::frontend::resolve::symbol_table::SymbolTable;
use crate::infra::arena::Arena;
use crate::infra::result::FelicoResult;
use crate::infra::shared_string::SharedString;
use crate::infra::source_span::SourceSpan;
use crate::interpret::value::InterpreterValue;
use crate::model::types::Type;
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Copy, Clone)]
pub struct Symbol<'ws> {
    inner: &'ws SymbolInner<'ws>,
}

#[derive(Debug)]
pub struct SymbolInner<'ws> {
    declaration_site: SourceSpan<'ws>,
    symbol_table: SymbolTable<'ws>,
    mutable: Mutex<&'ws mut SymbolMutableInner<'ws>>,
}

#[derive(Debug)]
pub struct SymbolMutableInner<'ws> {
    is_defined: bool,
    // Type of the symbol or expression
    ty: Type<'ws>,
    // Value of the expression
    value: Option<InterpreterValue<'ws>>,
}

impl<'ws> Symbol<'ws> {
    pub fn new(
        arena: &'ws Arena,
        declaration_site: SourceSpan<'ws>,
        is_defined: bool,
        ty: Type<'ws>,
        value: Option<InterpreterValue<'ws>>,
    ) -> Self {
        let mutable = arena.alloc(SymbolMutableInner {
            is_defined,
            ty,
            value,
        });
        let inner = arena.alloc(SymbolInner {
            declaration_site,
            symbol_table: SymbolTable::new(),
            mutable: Mutex::new(mutable),
        });
        Self { inner }
    }

    fn mutable<'a>(&'a self) -> MutexGuard<'a, &'ws mut SymbolMutableInner<'ws>> {
        self.inner.mutable.lock().unwrap()
    }

    pub fn type_signature(&self) -> String {
        self.mutable().ty.to_string()
    }

    pub fn add_symbol(&self, name: SharedString<'ws>, symbol: Symbol<'ws>) -> FelicoResult<()> {
        self.inner.symbol_table.insert(name, symbol);
        Ok(())
    }

    pub fn set_defined(&mut self, defined: bool) {
        self.mutable().is_defined = defined;
    }

    pub fn set_type(&mut self, ty: Type<'ws>) {
        self.mutable().ty = ty;
    }
    pub fn value(&self) -> Option<InterpreterValue<'ws>> {
        self.mutable().value.clone()
    }

    pub fn is_defined(&self) -> bool {
        self.mutable().is_defined
    }

    pub fn ty(&self) -> Type<'ws> {
        self.mutable().ty
    }

    pub fn declaration_site(&self) -> &SourceSpan<'ws> {
        &self.inner.declaration_site
    }

    pub fn lookup_symbol(&self, name: &str) -> Option<Symbol<'ws>> {
        self.inner.symbol_table.get(name)
    }
}
