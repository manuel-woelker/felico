use crate::module_builder::ConstantIndex;
use crate::op_code::OpCode;
use crate::operand::Operand;
use crate::slot::Slot;
use felico_base::result::FelicoResult;
use felico_base::test_print::TestPrint;
use std::fmt::{Debug, Formatter, Write};

#[repr(C)]
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Instruction {
    op_code: OpCode,
    operand_a: Operand,
    operand_b: Operand,
    operand_c: Operand,
}

pub const MAX_SLOT: u32 = 63;
pub const MAX_IMMEDIATE_CONST: u32 = 31;
pub const IMMEDIATE_CONST_PREFIX: u32 = 0b1100_0000;
pub const OPERAND_UNUSED: Operand = Operand::new(Slot::new(0));

impl Instruction {
    pub fn new(
        op_code: OpCode,
        operand_a: Operand,
        operand_b: Operand,
        operand_c: Operand,
    ) -> Self {
        Self {
            op_code,
            operand_a,
            operand_b,
            operand_c,
        }
    }

    pub fn new_constant(
        op_code: OpCode,
        operand_a: Operand,
        constant_index: ConstantIndex,
    ) -> FelicoResult<Self> {
        let index = constant_index.index();
        let operand_b = (index >> 8) as u8;
        let operand_c = (index & 0xff) as u8;
        Ok(Instruction::new(
            op_code,
            operand_a,
            Slot::from(operand_b).into(),
            Slot::from(operand_c).into(),
        ))
    }

    pub fn store_constant(dst_slot: Slot, constant_index: ConstantIndex) -> FelicoResult<Self> {
        Instruction::new_constant(OpCode::StoreConstant, dst_slot.into(), constant_index)
    }

    pub fn store_constant_length(
        dst_slot: Slot,
        constant_index: ConstantIndex,
    ) -> FelicoResult<Self> {
        Instruction::new_constant(OpCode::StoreConstantLength, dst_slot.into(), constant_index)
    }

    pub fn store_function(dst_slot: Slot, constant_index: ConstantIndex) -> FelicoResult<Self> {
        Instruction::new_constant(OpCode::StoreFunction, dst_slot.into(), constant_index)
    }

    pub fn ret() -> FelicoResult<Self> {
        Ok(Instruction::new(
            OpCode::Return,
            OPERAND_UNUSED,
            OPERAND_UNUSED,
            OPERAND_UNUSED,
        ))
    }

    pub fn call(fun_slot: Slot, return_slot: Slot) -> FelicoResult<Self> {
        Ok(Instruction::new(
            OpCode::Call,
            fun_slot.into(),
            return_slot.into(),
            OPERAND_UNUSED,
        ))
    }

    pub fn op_code(&self) -> OpCode {
        self.op_code
    }

    pub fn operand_a(&self) -> Operand {
        self.operand_a
    }

    pub fn operand_b(&self) -> Operand {
        self.operand_b
    }

    pub fn operand_c(&self) -> Operand {
        self.operand_c
    }

    pub fn operand_constant_index(&self) -> ConstantIndex {
        ConstantIndex::from(
            (self.operand_b().slot().index() as u16) << 8
                | (self.operand_c().slot().index() as u16),
        )
    }
}

impl Debug for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} {:?} {:?} {:?}",
            self.op_code(),
            self.operand_a(),
            self.operand_b(),
            self.operand_c()
        )?;
        Ok(())
    }
}

impl TestPrint for Instruction {
    fn test_print(&self, _write: &mut dyn Write, _indent: usize) -> FelicoResult<()> {
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
        };
        write!(
            write,
            "{:?} {} {} {}",
            self.op_code(),
            self.operand_a(),
            self.operand_b(),
            self.operand_c()
        )?;*/
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::instruction::Instruction;

    #[test]
    fn size_in_memory() {
        assert_eq!(size_of::<Instruction>(), 4);
    }
}
