use crate::frontend::ast::types::{Type, TypeKind};
use crate::infra::shared_string::Name;
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

pub struct ModuleManifest {
    pub name: Name,
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
            writeln!(f, "  {}: {}", entry.name, entry.ty)?;
        }
        Ok(())
    }
}

impl ModuleManifest {
    pub fn as_pretty_string(&self) -> String {
        use std::fmt::Write;
        let mut string = String::from("Module\n");
        for (_name, entry) in self.module_entries.iter().sorted_by_key(|(name, _)| *name) {
            writeln!(string, "  {}: {}", entry.name, entry.ty).unwrap();
            if let TypeKind::Struct(struct_type) = entry.ty.kind() {
                for (name, field) in struct_type.fields.iter().sorted_by_key(|(name, _)| *name) {
                    writeln!(string, "    {}: {}", name, field.ty).unwrap();
                }
            }
        }
        string
    }
}
