#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum OpCode {
    Move = 0,
    LoadConstant = 1,
    Call = 2,
}

const LAST_OPCODE: u8 = OpCode::Call as u8;

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        if value <= LAST_OPCODE {
            unsafe { std::mem::transmute::<u8, OpCode>(value) }
        } else {
            panic!("Invalid op code: {value}");
        }
    }
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
    fn u8_to_op_code() {
        assert_eq!(OpCode::Move, OpCode::from(0));
        assert_eq!(OpCode::LoadConstant, OpCode::from(1));
        assert_eq!(OpCode::Call, OpCode::from(2));
    }

    #[test]
    fn op_code_to_u8() {
        assert_eq!(u8::from(OpCode::Move), 0);
        assert_eq!(u8::from(OpCode::LoadConstant), 1);
        assert_eq!(u8::from(OpCode::Call), 2);
    }
}
