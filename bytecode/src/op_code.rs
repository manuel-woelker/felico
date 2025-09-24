#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum OpCode {
    StoreImmediate = 0,
    StoreConstant = 1,
    StoreConstantLength = 2,
    StoreFunction = 3,
    Call = 10,
    Return = 255,
}

impl From<OpCode> for u8 {
    fn from(op_code: OpCode) -> Self {
        op_code as u8
    }
}

#[cfg(test)]
mod tests {
    use crate::op_code::OpCode;

    #[test]
    fn op_code_to_u8() {
        assert_eq!(u8::from(OpCode::StoreImmediate), 0);
        assert_eq!(u8::from(OpCode::StoreConstant), 1);
        assert_eq!(u8::from(OpCode::Call), 10);
    }
}
