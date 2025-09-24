use crate::instruction::Instruction;
use crate::module::{ConstantPoolEntry, ConstantType, FunctionEntry, Module};
use crate::slot::Slot;
use felico_base::result::FelicoResult;

pub struct ModuleBuilder {
    name: String,
    constant_pool: Vec<ConstantPoolEntry>,
    functions: Vec<FunctionEntry>,
}

#[derive(Copy, Clone, Debug)]
pub struct ConstantIndex {
    index: u16,
}

impl ConstantIndex {
    pub fn new(index: u16) -> Self {
        Self { index }
    }

    pub fn index(&self) -> u16 {
        self.index
    }
}

impl From<u16> for ConstantIndex {
    fn from(value: u16) -> Self {
        Self { index: value }
    }
}

impl ModuleBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            constant_pool: vec![],
            functions: vec![],
        }
    }

    pub fn build(self) -> Module {
        Module {
            name: self.name,
            constant_pool: self.constant_pool,
            functions: self.functions,
        }
    }

    pub fn build_function(&mut self, name: impl Into<String>) -> FunctionBuilder<'_> {
        let name_constant = self.add_string(name);
        FunctionBuilder {
            module_builder: self,
            name_constant,
            instructions: vec![],
        }
    }

    pub fn add_string(&mut self, string: impl Into<String>) -> ConstantIndex {
        let string = string.into();
        let entry = ConstantPoolEntry::new(ConstantType::String, string);
        self.constant_pool.push(entry);
        ConstantIndex::new(self.constant_pool.len() as u16 - 1)
    }

    pub fn add_function_import(&mut self, function_name: impl Into<String>) -> ConstantIndex {
        let string = function_name.into();
        let entry = ConstantPoolEntry::new(ConstantType::FunctionImport, string);
        self.constant_pool.push(entry);
        ConstantIndex::new(self.constant_pool.len() as u16 - 1)
    }
}

pub struct FunctionBuilder<'module> {
    module_builder: &'module mut ModuleBuilder,
    pub name_constant: ConstantIndex,
    instructions: Vec<Instruction>,
}

impl FunctionBuilder<'_> {
    pub fn load_string(
        &mut self,
        ptr_dst_slot: Slot,
        length_dst_slot: Slot,
        string: impl Into<String>,
    ) -> FelicoResult<()> {
        let string_constant = self.module_builder.add_string(string);
        let instruction = Instruction::store_constant(ptr_dst_slot, string_constant)?;
        self.instructions.push(instruction);
        let instruction = Instruction::store_constant_length(length_dst_slot, string_constant)?;
        self.instructions.push(instruction);
        Ok(())
    }

    pub fn store_function(
        &mut self,
        dst_slot: Slot,
        function_index: ConstantIndex,
    ) -> FelicoResult<()> {
        let instruction = Instruction::store_function(dst_slot, function_index)?;
        self.instructions.push(instruction);
        Ok(())
    }

    pub fn call(&mut self, fun_slot: Slot, return_slot: Slot) -> FelicoResult<()> {
        let instruction = Instruction::call(fun_slot, return_slot)?;
        self.instructions.push(instruction);
        Ok(())
    }

    pub fn ret(&mut self) -> FelicoResult<()> {
        let instruction = Instruction::ret()?;
        self.instructions.push(instruction);
        Ok(())
    }
}

impl Drop for FunctionBuilder<'_> {
    fn drop(&mut self) {
        self.module_builder.functions.push(FunctionEntry::new(
            self.name_constant,
            std::mem::take(&mut self.instructions),
        ));
    }
}

#[cfg(test)]
mod tests {
    use crate::module_builder::ModuleBuilder;
    use crate::slot::Slot;
    use expect_test::expect;
    use felico_base::result::FelicoResult;
    use felico_base::test_print::TestPrint;

    #[test]
    fn test_new() -> FelicoResult<()> {
        let mut builder = ModuleBuilder::new("test");
        let print_constant_index = builder.add_function_import("print");
        let mut fbuilder = builder.build_function("main");
        fbuilder.load_string(Slot::from(13), Slot::from(14), "Hello World")?;
        fbuilder.store_function(Slot::from(3), print_constant_index)?;
        fbuilder.call(Slot::from(3), Slot::from(14))?;
        fbuilder.ret()?;
        drop(fbuilder);
        let module = builder.build();
        expect![[r#"
            Module test
              Constants:
                 0: FunctionImport <print>
                 1: String "main"
                 2: String "Hello World"
              Functions:
                 0: Function <main>
                   0: StoreConstant s13 c2 (String "Hello World")
                   1: StoreConstantLength s14 c2 (length: 11 bytes)
                   2: StoreFunction s3 c0 (FunctionImport <print>)
                   3: Call s3 s14 s0
                   4: Return s0 s0 s0
        "#]]
        .assert_eq(&module.test_print_to_string(0)?);
        Ok(())
    }
}
