use super::IntoDigits;
use crate::Digit;
use core::{iter, slice};

impl<'a> IntoDigits for &'a [Digit] {
    type Digits = iter::Copied<slice::Iter<'a, Digit>>;

    fn into_digits(self) -> Self::Digits {
        self.iter().copied()
    }
}
