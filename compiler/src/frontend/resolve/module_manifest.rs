use crate::infra::full_name::FullName;
use crate::infra::shared_string::Name;
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

#[derive(Debug)]
pub struct BundleManifest<'ws> {
    pub name: FullName<'ws>,
    pub modules: Vec<ModuleManifest<'ws>>,
}

pub struct ModuleManifest<'ws> {
    pub name: FullName<'ws>,
    pub module_entries: HashMap<Name<'ws>, ModuleEntry<'ws>>,
}

pub struct ModuleEntry<'ws> {
    pub name: Name<'ws>,
    pub type_signature: String,
}

impl<'ws> Debug for ModuleManifest<'ws> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Module\n")?;
        for (_name, entry) in self.module_entries.iter().sorted_by_key(|(name, _)| *name) {
            writeln!(f, "  {}: {}", entry.name, entry.type_signature)?;
        }
        Ok(())
    }
}

impl<'ws> ModuleManifest<'ws> {
    pub fn as_pretty_string(&self) -> String {
        use std::fmt::Write;
        let mut string = String::from("Module\n");
        for (_name, entry) in self.module_entries.iter().sorted_by_key(|(name, _)| *name) {
            writeln!(string, "  {}: {}", entry.name, entry.type_signature).unwrap();
            /*            if let TypeKind::Struct(struct_type) = entry.ty.kind() {
                for (name, field) in struct_type.fields.iter().sorted_by_key(|(name, _)| *name) {
                    writeln!(string, "    {}: {}", name, field.ty).unwrap();
                }
            }*/
        }
        string
    }
}
