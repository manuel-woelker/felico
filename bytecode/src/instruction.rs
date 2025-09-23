use crate::op_code::OpCode;
use felico_base::result::FelicoResult;
use felico_base::test_print::TestPrint;
use std::fmt::{Debug, Formatter, Write};

#[repr(C)]
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Instruction {
    byte_code: u32,
}

pub const MAX_SLOT: u32 = 63;
pub const MAX_IMMEDIATE_CONST: u32 = 31;
pub const IMMEDIATE_CONST_PREFIX: u32 = 0b1100_0000;

impl Instruction {
    pub fn new(byte_code: u32) -> Self {
        Self { byte_code }
    }

    pub fn op_code(&self) -> OpCode {
        OpCode::from((self.byte_code >> 24) as u8)
    }

    pub fn operand_a(&self) -> u8 {
        (self.byte_code >> 16 & 0xff) as u8
    }

    pub fn operand_b(&self) -> u8 {
        (self.byte_code >> 8 & 0xff) as u8
    }

    pub fn operand_c(&self) -> u8 {
        (self.byte_code & 0xff) as u8
    }

    pub fn byte_code(&self) -> u32 {
        self.byte_code
    }
}

impl Debug for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} {} {} {}",
            self.op_code(),
            self.operand_a(),
            self.operand_b(),
            self.operand_c()
        )
    }
}

impl TestPrint for Instruction {
    fn test_print(&self, write: &mut dyn Write, _indent: usize) -> FelicoResult<()> {
        /*        enum InstructionType {
            Wide,
            Narrow,
        };
        let instruction_type = match self.op_code() {
            OpCode::LoadConstant => {
                InstructionType::Wide
            }
            OpCode::Call => {
                InstructionType::Narrow
            }
            OpCode::Move => {
                InstructionType::Narrow
            }
        };*/
        write!(
            write,
            "{:?} {} {} {}",
            self.op_code(),
            self.operand_a(),
            self.operand_b(),
            self.operand_c()
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::instruction::Instruction;
    use crate::op_code::OpCode;
    use felico_base::test_print::TestPrint;

    #[test]
    fn size_in_memory() {
        assert_eq!(size_of::<Instruction>(), 4);
    }

    #[test]
    fn instruction_op_code() {
        let instruction = Instruction::new(0x01020304);
        assert_eq!(instruction.op_code(), OpCode::LoadConstant);
        assert_eq!(instruction.operand_a(), 0x02);
        assert_eq!(instruction.operand_b(), 0x03);
        assert_eq!(instruction.operand_c(), 0x04);
    }

    #[test]
    fn test_print() {
        let instruction = Instruction::new(0x01020304);
        assert_eq!(
            &instruction.test_print_to_string(0).unwrap(),
            "LoadConstant 2 3 4"
        );
    }
}
