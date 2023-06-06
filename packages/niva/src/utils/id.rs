
pub struct UniId(pub u8, pub u8);

impl From<u16> for UniId {
    fn from(value: u16) -> Self {
        UniId((value >> 8) as u8, value as u8)
    }
}

impl From<UniId> for u16 {
    fn from(value: UniId) -> Self {
        ((value.0 as u16) << 8) | (value.1 as u16)
    }
}