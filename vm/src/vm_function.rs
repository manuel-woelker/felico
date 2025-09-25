use crate::InstructionPointer;
use crate::native_function::{NativeFunction, NativeFunctionTrait};

pub enum VmFunctionKind {
    Instruction(InstructionPointer),
    Native(NativeFunction),
}

pub struct VmFunction {
    kind: VmFunctionKind,
}

impl VmFunction {
    pub fn from_instruction(instruction_start: InstructionPointer) -> Self {
        Self {
            kind: VmFunctionKind::Instruction(instruction_start),
        }
    }

    pub fn from_native(function: impl NativeFunctionTrait + 'static) -> Self {
        Self {
            kind: VmFunctionKind::Native(NativeFunction::new(function)),
        }
    }

    pub fn kind(&self) -> &VmFunctionKind {
        &self.kind
    }

    pub fn kind_mut(&mut self) -> &mut VmFunctionKind {
        &mut self.kind
    }
}
