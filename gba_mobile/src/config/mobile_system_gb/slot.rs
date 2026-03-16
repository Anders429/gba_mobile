use super::PhoneNumber;

#[derive(Debug)]
pub struct Slot {
    pub phone_number: PhoneNumber,
    pub id: [u8; 16],
}
