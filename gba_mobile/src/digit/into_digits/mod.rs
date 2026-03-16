pub mod ipv4addr;

use super::Digit;

pub trait IntoDigits {
    type Digits: Iterator<Item = Digit>;

    fn into_digits(self) -> Self::Digits;
}
