use crate::vm_function::VmFunction;
use felico_arena::typed_arena::{TypedArena, TypedArenaHandle};
use felico_base::result::FelicoResult;
use felico_base::{bail, err};
use std::collections::HashMap;

pub struct FunctionArena {
    vm_functions: TypedArena<VmFunction>,
    function_name_map: HashMap<String, TypedArenaHandle<VmFunction>>,
}

pub type FunctionHandle = TypedArenaHandle<VmFunction>;

impl FunctionArena {
    pub fn new() -> Self {
        Self {
            vm_functions: TypedArena::new(),
            function_name_map: HashMap::new(),
        }
    }

    pub fn add_function(&mut self, function: VmFunction) -> FelicoResult<FunctionHandle> {
        let name = function.name().to_string();
        let name_clone = name.clone();
        let function_handle = self.vm_functions.add(function)?;
        let previous_value = self.function_name_map.insert(name, function_handle);
        if previous_value.is_some() {
            bail!("Function with name '{name_clone}' already exists");
        }
        Ok(function_handle)
    }

    pub fn get_function_handle(&self, name: &str) -> FelicoResult<FunctionHandle> {
        self.function_name_map
            .get(name)
            .copied()
            .ok_or_else(|| err!("Function with name '{name}' not found"))
    }

    pub fn get_function(&self, handle: FunctionHandle) -> FelicoResult<&VmFunction> {
        self.vm_functions.get(handle)
    }
}

impl Default for FunctionArena {
    fn default() -> Self {
        Self::new()
    }
}
