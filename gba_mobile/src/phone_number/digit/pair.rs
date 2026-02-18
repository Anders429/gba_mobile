use super::Digit;
use core::{
    fmt,
    fmt::{Debug, Formatter},
};
use deranged::RangedU8;

/// A pair of digits stored in a phone number.
///
/// These are packed optional `Digit`s, stored two to a byte. If a halfbyte does not represent a
/// valid digit, it is considered to represent no digit.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct Pair(u8);

impl Pair {
    pub(crate) fn to_digits(self) -> [Option<Digit>; 2] {
        [self.0 & 0xf, (self.0 >> 4) & 0xf].map(|halfbyte| RangedU8::new(halfbyte).map(Digit::new))
    }

    pub(crate) fn from_digits(digits: [Option<Digit>; 2]) -> Self {
        fn to_raw(digit: Option<Digit>) -> u8 {
            digit.map(|digit| digit.0.get()).unwrap_or(0xf)
        }

        Self(to_raw(digits[0]) | (to_raw(digits[1]) << 4))
    }
}

impl Debug for Pair {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        let digits = self.to_digits();
        formatter
            .debug_tuple("Pair")
            .field(&digits[0])
            .field(&digits[1])
            .finish()
    }
}

impl Default for Pair {
    fn default() -> Self {
        // Default to two empty digits.
        Self(0xff)
    }
}

#[cfg(test)]
mod tests {
    use super::Pair;
    use crate::phone_number::Digit;
    use alloc::format;
    use claims::assert_ok;
    use gba_test::test;

    #[test]
    fn debug_none() {
        assert_eq!(
            format!("{:?}", Pair::from_digits([None, None])),
            "Pair(None, None)"
        );
    }

    #[test]
    fn debug_first() {
        assert_eq!(
            format!(
                "{:?}",
                Pair::from_digits([Some(assert_ok!(Digit::try_from('1'))), None])
            ),
            "Pair(Some(1), None)"
        );
    }

    #[test]
    fn debug_second() {
        assert_eq!(
            format!(
                "{:?}",
                Pair::from_digits([None, Some(assert_ok!(Digit::try_from('#')))])
            ),
            "Pair(None, Some(#))"
        );
    }

    #[test]
    fn debug_both() {
        assert_eq!(
            format!(
                "{:?}",
                Pair::from_digits([
                    Some(assert_ok!(Digit::try_from('4'))),
                    Some(assert_ok!(Digit::try_from('*')))
                ])
            ),
            "Pair(Some(4), Some(*))"
        );
    }

    #[test]
    fn to_digits_none() {
        let digits = [None, None];
        let pair = Pair::from_digits(digits);
        assert_eq!(pair.to_digits(), digits);
    }

    #[test]
    fn to_digits_first() {
        let digits = [Some(assert_ok!(Digit::try_from('5'))), None];
        let pair = Pair::from_digits(digits);
        assert_eq!(pair.to_digits(), digits);
    }

    #[test]
    fn to_digits_second() {
        let digits = [None, Some(assert_ok!(Digit::try_from('#')))];
        let pair = Pair::from_digits(digits);
        assert_eq!(pair.to_digits(), digits);
    }

    #[test]
    fn to_digits_both() {
        let digits = [
            Some(assert_ok!(Digit::try_from('7'))),
            Some(assert_ok!(Digit::try_from('1'))),
        ];
        let pair = Pair::from_digits(digits);
        assert_eq!(pair.to_digits(), digits);
    }

    #[test]
    fn default() {
        assert_eq!(Pair::default(), Pair::from_digits([None, None]));
    }
}
