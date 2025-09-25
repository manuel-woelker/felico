use crate::vm::VM;
use felico_base::result::FelicoResult;

pub struct NativeFunction {
    function: Box<dyn NativeFunctionTrait + 'static>,
}

impl NativeFunction {
    pub fn new(function: impl NativeFunctionTrait + 'static) -> Self {
        Self {
            function: Box::new(function),
        }
    }

    pub fn call(&self, vm: &mut VM) -> FelicoResult<()> {
        self.function.call(vm)
    }
}

pub trait NativeFunctionTrait {
    fn call(&self, vm: &mut VM) -> FelicoResult<()>;
}

impl<T: Fn(&mut VM) -> FelicoResult<()>> NativeFunctionTrait for T {
    fn call(&self, vm: &mut VM) -> FelicoResult<()> {
        self(vm)
    }
}
