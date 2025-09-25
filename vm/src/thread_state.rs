use crate::InstructionPointer;
use felico_bytecode::operand::Operand;

#[derive(Debug, Default)]
pub struct ThreadState {
    /// Program counter, points to current instruction
    pc: InstructionPointer,
    /// Stack of values
    stack: Vec<u64>,
    /// Slot offset of the current instruction, i.e. frame pointer
    slot_offset: usize,
    /// Call stack
    call_stack: Vec<Frame>,
}

impl ThreadState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stack(&self) -> &Vec<u64> {
        &self.stack
    }

    pub fn stack_mut(&mut self) -> &mut Vec<u64> {
        &mut self.stack
    }
    pub fn set_instruction_pointer(&mut self, pc: InstructionPointer) {
        self.pc = pc;
    }

    pub fn instruction_pointer(&self) -> InstructionPointer {
        self.pc
    }

    pub fn push_frame(&mut self, frame: Frame) {
        self.call_stack.push(frame);
    }

    pub fn pop_frame(&mut self) -> Frame {
        self.call_stack.pop().unwrap()
    }

    pub fn current_frame(&self) -> &Frame {
        self.call_stack.last().unwrap()
    }

    pub fn set_slot(&mut self, operand: Operand, value: u64) {
        let slot_index = operand.slot().index() as usize + self.slot_offset;
        self.stack[slot_index] = value;
    }

    pub fn get_slot(&self, operand: Operand) -> u64 {
        let slot_index = operand.slot().index() as usize + self.slot_offset;
        self.stack[slot_index]
    }

    pub fn set_slot_offset(&mut self, slot_offset: usize) {
        self.slot_offset = slot_offset;
    }

    pub fn slot_offset(&self) -> usize {
        self.slot_offset
    }
}

#[derive(Debug, Default)]
pub struct Frame {
    function_index: usize,
}

impl Frame {
    pub fn new(function_index: usize) -> Self {
        Self { function_index }
    }

    pub fn function_index(&self) -> usize {
        self.function_index
    }
}
