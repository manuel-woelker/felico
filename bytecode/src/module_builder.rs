use crate::instruction::{IMMEDIATE_CONST_PREFIX, MAX_IMMEDIATE_CONST, MAX_SLOT};
use crate::module::{ConstantPoolEntry, ConstantType, FunctionEntry, Module};
use crate::op_code::OpCode;

pub struct ModuleBuilder {
    name: String,
    data_pool: Vec<u8>,
    constant_pool: Vec<ConstantPoolEntry>,
    instruction_pool: Vec<u32>,
    functions: Vec<FunctionEntry>,
}

impl ModuleBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_pool: vec![],
            constant_pool: vec![],
            instruction_pool: vec![],
            functions: vec![],
        }
    }

    pub fn build(self) -> Module {
        Module {
            name: self.name,
            data_pool: self.data_pool,
            constant_pool: self.constant_pool,
            instruction_pool: self.instruction_pool,
            functions: self.functions,
        }
    }

    pub fn build_function(&mut self, name: impl Into<String>) -> FunctionBuilder<'_> {
        let name_constant = self.add_string(name);
        let instruction_offset = self.instruction_pool.len() as u32;
        FunctionBuilder {
            module_builder: self,
            name_constant,
            instruction_offset,
        }
    }

    pub fn add_string(&mut self, string: impl Into<String>) -> u32 {
        let string = string.into();
        let offset = self.data_pool.len();
        self.data_pool.extend_from_slice(string.as_bytes());
        let length = self.data_pool.len() - offset;
        let entry = ConstantPoolEntry::new(ConstantType::String, offset as u32, length as u32);
        self.constant_pool.push(entry);
        self.constant_pool.len() as u32 - 1
    }
}

pub struct FunctionBuilder<'module> {
    module_builder: &'module mut ModuleBuilder,
    pub name_constant: u32,
    pub instruction_offset: u32,
}

impl<'module> FunctionBuilder<'module> {
    pub fn load_string(&mut self, dst_slot: u32, string: impl Into<String>) {
        assert!(dst_slot < MAX_SLOT);
        let string_constant = self.module_builder.add_string(string);
        assert!(string_constant < MAX_IMMEDIATE_CONST);
        let byte_code = (OpCode::LoadConstant as u32) << 24
            | (dst_slot) << 16
            | (string_constant | IMMEDIATE_CONST_PREFIX) << 8;
        self.module_builder.instruction_pool.push(byte_code);
    }
}

impl Drop for FunctionBuilder<'_> {
    fn drop(&mut self) {
        self.module_builder.functions.push(FunctionEntry::new(
            self.name_constant,
            self.instruction_offset,
            self.module_builder.instruction_pool.len() as u32 - self.instruction_offset,
        ));
    }
}

#[cfg(test)]
mod tests {
    use crate::module_builder::ModuleBuilder;
    use expect_test::expect;
    use felico_base::result::FelicoResult;
    use felico_base::test_print::TestPrint;

    #[test]
    fn test_new() -> FelicoResult<()> {
        let mut builder = ModuleBuilder::new("test");
        let mut fbuilder = builder.build_function("main");
        fbuilder.load_string(13, "Hello World");
        drop(fbuilder);
        let module = builder.build();
        expect![[r#"
            Module test
              Constants:
                 0: String "main"
                 1: String "Hello World"
              Functions:
                 0: Function <main>
                   0: LoadConstant s13 c1 "Hello World" s0
        "#]]
        .assert_eq(&module.test_print_to_string(0)?);
        Ok(())
    }
}
