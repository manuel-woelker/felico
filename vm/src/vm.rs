use crate::thread_state::{Frame, ThreadState};
use crate::vm_function::VmFunction;
use felico_base::result::FelicoResult;
use felico_base::{bail, err};
use felico_bytecode::instruction::Instruction;
use felico_bytecode::module::{ConstantPoolEntry, Module};
use felico_bytecode::op_code::OpCode;
use std::collections::HashMap;

pub struct VM {
    constant_pool: Vec<ConstantPoolEntry>,
    instructions: Vec<Instruction>,
    vm_functions: Vec<VmFunction>,
    function_name_map: HashMap<String, usize>,
    // TODO: arena
    thread_state: ThreadState,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        Self {
            thread_state: ThreadState::default(),
            instructions: Vec::new(),
            constant_pool: Vec::new(),
            vm_functions: Vec::new(),
            function_name_map: HashMap::new(),
        }
    }

    pub fn load_module(&mut self, module: Module) -> FelicoResult<()> {
        let mut instruction_offset = self.instructions.len();
        // TODO: constant pool offset
        // let constant_pool_offset = self.constant_pool.len();
        let vm_functions = &mut self.vm_functions;
        for function in &module.functions {
            let vm_function = VmFunction::new(instruction_offset);
            self.instructions.extend(function.instructions());
            instruction_offset += function.instructions().len();
            let function_name = module
                .get_constant(function.name_constant())?
                .as_str()?
                .to_string();
            self.function_name_map
                .insert(function_name, vm_functions.len());
            vm_functions.push(vm_function);
        }
        self.constant_pool.extend(module.constant_pool);
        //        self.modules.push(module);
        Ok(())
    }

    pub fn run(&mut self) -> FelicoResult<()> {
        self.prepare_run()?;
        self.execute()?;
        Ok(())
    }

    fn prepare_run(&mut self) -> FelicoResult<()> {
        // find main function
        let main_function_index = *self
            .function_name_map
            .get("main")
            .ok_or_else(|| err!("Main function not found"))?;
        let main_function = self
            .vm_functions
            .get(main_function_index)
            .ok_or_else(|| err!("Main function not present"))?;
        self.thread_state
            .set_instruction_pointer(main_function.instruction_start());
        self.thread_state
            .push_frame(Frame::new(main_function_index));
        self.thread_state.stack_mut().resize(100, 0);
        Ok(())
    }

    fn execute(&mut self) -> FelicoResult<()> {
        loop {
            let pc = self.thread_state.instruction_pointer();
            let instruction = self.instructions[pc];
            match instruction.op_code() {
                OpCode::StoreConstant => {
                    let target_slot = instruction.operand_a();
                    let constant_index = instruction.operand_constant_index();
                    self.thread_state
                        .set_slot(target_slot, constant_index.index() as u64);
                }
                OpCode::StoreConstantLength => {
                    let target_slot = instruction.operand_a();
                    let constant_index = instruction.operand_constant_index();
                    let constant = self
                        .constant_pool
                        .get(constant_index.index() as usize)
                        .ok_or_else(|| {
                            err!("Constant index out of bounds: {:?}", constant_index)
                        })?;
                    dbg!(&constant.data.len());
                    dbg!(target_slot);
                    self.thread_state
                        .set_slot(target_slot, constant.data.len() as u64);
                }
                OpCode::StoreFunction => {
                    // TODO StoreFunction implement
                }
                OpCode::Call => {
                    // TODO Call implement
                }
                OpCode::Return => {
                    return Ok(());
                }
                other => bail!("Unimplemented opcode: {:?}", other),
            }
            self.thread_state.set_instruction_pointer(pc + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::VM;
    use felico_base::result::FelicoResult;
    use felico_bytecode::module_builder::ModuleBuilder;
    use felico_bytecode::slot::Slot;

    #[test]
    fn test_run() -> FelicoResult<()> {
        let mut builder = ModuleBuilder::new("test");
        let print_constant_index = builder.add_function_import("print");
        let mut fbuilder = builder.build_function("main");
        fbuilder.load_string(Slot::from(3), Slot::from(4), "Hello World")?;
        fbuilder.store_function(Slot::from(2), print_constant_index)?;
        fbuilder.call(Slot::from(2), Slot::from(3))?;
        fbuilder.ret()?;
        drop(fbuilder);
        let module = builder.build();

        let mut vm = VM::new();
        vm.load_module(module)?;
        vm.run()?;
        println!("Stack: {:?}", &vm.thread_state.stack()[0..10]);
        Ok(())
    }
}
