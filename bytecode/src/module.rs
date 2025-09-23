use crate::instruction::{IMMEDIATE_CONST_PREFIX, Instruction, MAX_SLOT};
use felico_base::result::FelicoResult;
use felico_base::test_print::TestPrint;
use std::fmt::Write;

pub struct Module {
    pub name: String,
    pub data_pool: Vec<u8>,
    pub constant_pool: Vec<ConstantPoolEntry>,
    pub functions: Vec<FunctionEntry>,
    pub instruction_pool: Vec<u32>,
}

impl Module {
    pub fn get_string_constant_by_entry(&self, entry: &ConstantPoolEntry) -> FelicoResult<&str> {
        Ok(std::str::from_utf8(
            &self.data_pool[entry.offset()..entry.offset() + entry.length()],
        )?)
    }
}

pub struct ConstantPoolEntry {
    pub length: u32,
    pub offset: u32,
}

impl ConstantPoolEntry {
    pub fn new(constant_type: ConstantType, offset: u32, length: u32) -> Self {
        assert!(length < 0xffffff);
        Self {
            length: length & 0xffffff | ((constant_type as u32) << 24),
            offset,
        }
    }

    pub fn constant_type(&self) -> ConstantType {
        ConstantType::from((self.length >> 24) as u8)
    }

    pub fn offset(&self) -> usize {
        self.offset as usize
    }

    pub fn length(&self) -> usize {
        (self.length & 0xffffff) as usize
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ConstantType {
    ByteArray = 0,
    String = 1,
    FunctionImport = 2,
}

impl From<u8> for ConstantType {
    fn from(value: u8) -> Self {
        assert!(value <= 2);
        unsafe { std::mem::transmute(value) }
    }
}

pub struct FunctionEntry {
    pub name_constant: u32,
    pub instruction_offset: u32,
    pub instruction_length: u32,
}

impl FunctionEntry {
    pub fn new(name_constant: u32, instruction_offset: u32, instruction_length: u32) -> Self {
        Self {
            name_constant,
            instruction_offset,
            instruction_length,
        }
    }
}

impl TestPrint for Module {
    fn test_print(&self, write: &mut dyn Write, _indent: usize) -> FelicoResult<()> {
        writeln!(write, "Module {}", self.name)?;
        writeln!(write, "  Constants:")?;
        for (index, constant) in self.constant_pool.iter().enumerate() {
            write!(write, "   {index:3}: ")?;
            match constant.constant_type() {
                ConstantType::ByteArray => {
                    writeln!(write, "ByteArray ({} bytes)", constant.length())?;
                }
                ConstantType::String => {
                    let string = self.get_string_constant_by_entry(constant)?;
                    writeln!(write, "String \"{string}\"")?;
                }
                ConstantType::FunctionImport => {
                    let string = self.get_string_constant_by_entry(constant)?;
                    writeln!(write, "FunctionImport <{string}>")?;
                }
            }
        }
        writeln!(write, "  Functions:")?;
        for (index, function) in self.functions.iter().enumerate() {
            write!(write, "   {index:3}: ")?;
            let name_constant = &self.constant_pool[function.name_constant as usize];
            let name = std::str::from_utf8(
                &self.data_pool
                    [name_constant.offset()..name_constant.offset() + name_constant.length()],
            )?;
            writeln!(write, "Function <{name}>")?;
            let instructions = &self.instruction_pool[function.instruction_offset as usize
                ..function.instruction_offset as usize + function.instruction_length as usize];
            let mut iter = instructions.iter().enumerate();
            loop {
                let Some((index, byte_code)) = iter.next() else {
                    break;
                };
                write!(write, "     {index:3}: ")?;
                let instruction = Instruction::new(*byte_code);
                write!(write, "{:?}", instruction.op_code())?;
                let module = &self;
                let write_operand = |write: &mut dyn Write, operand: u8| -> FelicoResult<()> {
                    if operand as u32 <= MAX_SLOT {
                        write!(write, " s{operand}")?
                    } else if operand as u32 & IMMEDIATE_CONST_PREFIX != 0 {
                        let immediate = operand as u32 & !IMMEDIATE_CONST_PREFIX;
                        let constant = &module.constant_pool[immediate as usize];
                        let string = module.get_string_constant_by_entry(constant)?;
                        write!(write, " c{immediate} \"{string}\"")?
                    }
                    Ok(())
                };
                write_operand(write, instruction.operand_a())?;
                write_operand(write, instruction.operand_b())?;
                write_operand(write, instruction.operand_c())?;
                writeln!(write)?;
            }
        }
        Ok(())
    }
}
