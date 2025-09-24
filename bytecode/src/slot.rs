#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Slot {
    slot_index: u8,
}

impl Slot {
    pub const fn new(slot_index: u8) -> Self {
        Self { slot_index }
    }
}

impl From<u8> for Slot {
    fn from(slot_index: u8) -> Self {
        Slot { slot_index }
    }
}

impl Slot {
    pub fn index(&self) -> u8 {
        self.slot_index
    }
}
