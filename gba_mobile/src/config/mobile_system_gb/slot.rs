use super::PhoneNumber;

#[derive(Clone, Debug, Default)]
pub struct Slot {
    pub phone_number: PhoneNumber,
    pub id: [u8; 16],
}
