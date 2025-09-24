use crate::instruction::Instruction;
use crate::module_builder::ConstantIndex;
use crate::op_code::OpCode;
use crate::operand::Operand;
use felico_base::result::FelicoResult;
use felico_base::test_print::TestPrint;
use felico_base::{bail, err};
use std::fmt::Write;

pub struct Module {
    pub name: String,
    pub constant_pool: Vec<ConstantPoolEntry>,
    pub functions: Vec<FunctionEntry>,
}

impl Module {
    pub fn get_constant(&self, constant_index: ConstantIndex) -> FelicoResult<&ConstantPoolEntry> {
        self.constant_pool
            .get(constant_index.index() as usize)
            .ok_or_else(|| err!("Constant index out of bounds: {}", constant_index.index()))
    }
}

pub struct ConstantPoolEntry {
    constant_type: ConstantType,
    pub data: Vec<u8>,
}

impl ConstantPoolEntry {
    pub fn new(constant_type: ConstantType, data: impl Into<Vec<u8>>) -> Self {
        Self {
            constant_type,
            data: data.into(),
        }
    }

    pub fn constant_type(&self) -> ConstantType {
        self.constant_type
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn as_str(&self) -> FelicoResult<&str> {
        if self.constant_type != ConstantType::String {
            bail!("Constant is not a string, but {:?}", self.constant_type)
        }
        Ok(std::str::from_utf8(&self.data)?)
    }

    pub fn as_function_import(&self) -> FelicoResult<&str> {
        if self.constant_type != ConstantType::FunctionImport {
            bail!(
                "Constant is not a FunctionImport, but {:?}",
                self.constant_type
            )
        }
        Ok(std::str::from_utf8(&self.data)?)
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ConstantType {
    ByteArray = 0,
    String = 1,
    FunctionImport = 2,
}

pub struct FunctionEntry {
    pub name_constant: ConstantIndex,
    instructions: Vec<Instruction>,
}

impl FunctionEntry {
    pub fn new(name_constant: ConstantIndex, instructions: Vec<Instruction>) -> Self {
        Self {
            name_constant,
            instructions,
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
                    writeln!(write, "ByteArray ({} bytes)", constant.data().len())?;
                }
                ConstantType::String => {
                    let string = constant.as_str()?;
                    writeln!(write, "String \"{string}\"")?;
                }
                ConstantType::FunctionImport => {
                    let string = constant.as_function_import()?;
                    writeln!(write, "FunctionImport <{string}>")?;
                }
            }
        }
        writeln!(write, "  Functions:")?;
        for (index, function) in self.functions.iter().enumerate() {
            write!(write, "   {index:3}: ")?;
            let function_name = self.get_constant(function.name_constant)?.as_str()?;
            writeln!(write, "Function <{function_name}>")?;
            let instructions = &function.instructions;
            let mut iter = instructions.iter().enumerate();
            loop {
                let Some((index, instruction)) = iter.next() else {
                    break;
                };
                write!(write, "     {index:3}: ")?;
                write!(write, "{:?}", instruction.op_code())?;
                let write_operand = |write: &mut dyn Write, operand: Operand| -> FelicoResult<()> {
                    write!(write, " s{}", operand.slot().index())?;
                    Ok(())
                };
                let write_constant =
                    |write: &mut dyn Write, constant_index: ConstantIndex| -> FelicoResult<()> {
                        let constant = self.get_constant(constant_index)?;
                        match constant.constant_type {
                            ConstantType::String => {
                                let string = constant.as_str()?;
                                write!(
                                    write,
                                    " c{} (String \"{}\")",
                                    constant_index.index(),
                                    string
                                )?;
                            }
                            ConstantType::FunctionImport => {
                                let string = constant.as_function_import()?;
                                write!(
                                    write,
                                    " c{} (FunctionImport <{}>)",
                                    constant_index.index(),
                                    string
                                )?;
                            }
                            ConstantType::ByteArray => {
                                write!(
                                    write,
                                    " c{} ({} bytes)",
                                    constant_index.index(),
                                    constant.data.len()
                                )?;
                            }
                        }
                        Ok(())
                    };
                write_operand(write, instruction.operand_a())?;
                match instruction.op_code() {
                    OpCode::StoreConstant | OpCode::StoreFunction => {
                        let constant_index = instruction.operand_constant_index();
                        write_constant(write, constant_index)?;
                    }
                    OpCode::StoreConstantLength => {
                        let constant_index = instruction.operand_constant_index();
                        let constant = self.get_constant(constant_index)?;
                        write!(
                            write,
                            " c{} (length: {} bytes)",
                            constant_index.index(),
                            constant.data.len()
                        )?;
                    }
                    _ => {
                        write_operand(write, instruction.operand_b())?;
                        write_operand(write, instruction.operand_c())?;
                    }
                }
                writeln!(write)?;
            }
        }
        Ok(())
    }
}
