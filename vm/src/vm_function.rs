use crate::InstructionPointer;

pub struct VmFunction {
    instruction_start: InstructionPointer,
}

impl VmFunction {
    pub fn new(instruction_start: InstructionPointer) -> Self {
        Self { instruction_start }
    }

    pub fn instruction_start(&self) -> InstructionPointer {
        self.instruction_start
    }
}
