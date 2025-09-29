use crate::function_arena::{FunctionArena, FunctionHandle};
use crate::native_function::NativeFunctionTrait;
use crate::thread_state::{Frame, ThreadState};
use crate::vm_function::{VmFunction, VmFunctionKind};
use felico_base::result::FelicoResult;
use felico_base::{bail, err};
use felico_bytecode::instruction::Instruction;
use felico_bytecode::module::{ConstantPoolEntry, ConstantType, Module};
use felico_bytecode::op_code::OpCode;
use std::collections::HashMap;

pub struct VM {
    function_arena: FunctionArena,
    constant_pool: Vec<ConstantPoolEntry>,
    instructions: Vec<Instruction>,
    // map from function import constant index to function handle
    function_handle_map: HashMap<u32, FunctionHandle>,
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
            function_arena: FunctionArena::new(),
            thread_state: ThreadState::default(),
            instructions: Vec::new(),
            constant_pool: Vec::new(),
            function_handle_map: HashMap::new(),
        }
    }

    pub fn register_native_function(
        &mut self,
        name: &str,
        function: impl NativeFunctionTrait + 'static,
    ) -> FelicoResult<FunctionHandle> {
        let function = VmFunction::from_native(name, function);
        self.function_arena.add_function(function)
    }

    pub fn load_module(&mut self, module: Module) -> FelicoResult<()> {
        // TODO: constant pool offset
        // let constant_pool_offset = self.constant_pool.len();
        for function in &module.functions {
            let instruction_offset = self.instructions.len();
            self.instructions.extend(function.instructions());
            let function_name = module
                .get_constant(function.name_constant())?
                .as_str()?
                .to_string();
            let vm_function = VmFunction::from_instruction(function_name, instruction_offset);
            self.function_arena.add_function(vm_function)?;
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
        for (index, constant) in self.constant_pool.iter().enumerate() {
            if constant.constant_type() == ConstantType::FunctionImport {
                // Lookup function name
                let function_name = constant.as_function_import()?;
                let function_handle = self.function_arena.get_function_handle(function_name)?;
                self.function_handle_map
                    .insert(index as u32, function_handle);
            }
        }
        // find main function
        let main_function_handle = self.function_arena.get_function_handle("main")?;
        let main_function = self.function_arena.get_function(main_function_handle)?;
        let VmFunctionKind::Instruction(instruction_start) = &main_function.kind() else {
            bail!("Main function is not an instruction function");
        };
        self.thread_state
            .set_instruction_pointer(*instruction_start);
        self.thread_state
            .push_frame(Frame::new(main_function_handle));
        self.thread_state.stack_mut().resize(100, 0);
        Ok(())
    }

    fn execute(&mut self) -> FelicoResult<()> {
        let function_arena = std::mem::take(&mut self.function_arena);
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
                    self.thread_state
                        .set_slot(target_slot, constant.data.len() as u64);
                }
                OpCode::StoreFunction => {
                    let target_slot = instruction.operand_a();
                    let constant_index = instruction.operand_constant_index();
                    let function_handle = self
                        .function_handle_map
                        .get(&(constant_index.index() as u32))
                        .ok_or_else(|| {
                            err!("Constant index out of bounds: {:?}", constant_index)
                        })?;
                    self.thread_state
                        .set_slot(target_slot, (*function_handle).into());
                }
                OpCode::Call => {
                    let function_slot = instruction.operand_a();
                    let function_index = self.thread_state.get_slot(function_slot);
                    let function_handle = FunctionHandle::from(function_index);
                    let argument_slot = instruction.operand_b();
                    self.thread_state.set_slot_offset(
                        self.thread_state.slot_offset() + argument_slot.slot().index() as usize,
                    );
                    self.thread_state.push_frame(Frame::new(function_handle));

                    let function = function_arena.get_function(function_handle)?;
                    match function.kind() {
                        VmFunctionKind::Native(native_function) => {
                            native_function.call(self)?;
                        }
                        VmFunctionKind::Instruction(instruction_start) => {
                            self.thread_state
                                .set_instruction_pointer(*instruction_start);
                        }
                    }
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
    use felico_base::err;
    use felico_base::result::FelicoResult;
    use felico_base::test_print::TestPrint;
    use felico_bytecode::module_builder::ModuleBuilder;
    use felico_bytecode::operand::Operand;
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
        println!("Module: {}", module.test_print_to_string(0)?);

        let mut vm = VM::new();
        vm.register_native_function("print", |vm: &mut VM| {
            let string_ptr = vm.thread_state.get_slot(Operand::from(Slot::from(0)));
            let string_length = vm.thread_state.get_slot(Operand::from(Slot::from(1)));
            let constant = vm
                .constant_pool
                .get(string_ptr as usize)
                .ok_or_else(|| err!("String index out of bounds: {:?}", string_ptr))?;
            let string = &constant.as_str()?[0..string_length as usize];
            println!("PRINT! {string_ptr} {string_length}");
            println!("{}", string);
            //            println!("{}", string);
            Ok(())
        })?;
        vm.load_module(module)?;
        vm.run()?;
        println!("Stack: {:?}", &vm.thread_state.stack()[0..10]);
        Ok(())
    }
}
