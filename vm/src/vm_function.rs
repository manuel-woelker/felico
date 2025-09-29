use crate::InstructionPointer;
use crate::native_function::{NativeFunction, NativeFunctionTrait};

pub enum VmFunctionKind {
    Instruction(InstructionPointer),
    Native(NativeFunction),
}

pub struct VmFunction {
    name: String,
    kind: VmFunctionKind,
}

impl VmFunction {
    pub fn from_instruction(
        name: impl Into<String>,
        instruction_start: InstructionPointer,
    ) -> Self {
        Self {
            name: name.into(),
            kind: VmFunctionKind::Instruction(instruction_start),
        }
    }

    pub fn from_native(
        name: impl Into<String>,
        function: impl NativeFunctionTrait + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            kind: VmFunctionKind::Native(NativeFunction::new(function)),
        }
    }

    pub fn kind(&self) -> &VmFunctionKind {
        &self.kind
    }

    pub fn kind_mut(&mut self) -> &mut VmFunctionKind {
        &mut self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
