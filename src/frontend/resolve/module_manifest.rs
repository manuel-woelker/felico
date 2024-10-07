use crate::frontend::ast::types::Type;
use crate::infra::shared_string::Name;
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

pub struct ModuleManifest {
    // TODO Module name
    pub module_entries: HashMap<Name, ModuleEntry>,
}

pub struct ModuleEntry {
    pub name: Name,
    pub ty: Type,
}

impl Debug for ModuleManifest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Module\n")?;
        for (_name, entry) in self.module_entries.iter().sorted_by_key(|(name, _)| *name) {
            write!(f, "  {}: {}\n", entry.name, entry.ty)?;
        }
        Ok(())
    }
}
