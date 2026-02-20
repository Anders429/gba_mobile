pub mod ipv4addr;

use crate::phone_number::Digit;

pub trait IntoDigits {
    type Digits: Iterator<Item = Digit>;

    fn into_digits(self) -> Self::Digits;
}
