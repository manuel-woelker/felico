use crate::slot::Slot;

/// An operand to an instruction, usually a slot
///
///
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Operand {
    slot: Slot,
}

impl Operand {
    pub const fn new(slot: Slot) -> Self {
        Self { slot }
    }

    pub fn slot(&self) -> Slot {
        self.slot
    }
}

impl From<Slot> for Operand {
    fn from(slot: Slot) -> Self {
        Self { slot }
    }
}
