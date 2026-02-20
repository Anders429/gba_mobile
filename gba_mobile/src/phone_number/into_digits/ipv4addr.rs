use super::IntoDigits;
use crate::phone_number::Digit;
use core::{array, net::Ipv4Addr};
use deranged::RangedU8;

impl IntoDigits for Ipv4Addr {
    type Digits = DigitsIter;

    fn into_digits(self) -> Self::Digits {
        DigitsIter {
            octets: self.octets().into_iter(),
            current: None,
        }
    }
}

enum Octet {
    Hundreds(u8),
    Tens(u8),
    Ones(u8),
}

pub struct DigitsIter {
    octets: array::IntoIter<u8, 4>,
    current: Option<Octet>,
}

impl Iterator for DigitsIter {
    type Item = Digit;

    fn next(&mut self) -> Option<Self::Item> {
        let digit = match self
            .current
            .take()
            .or_else(|| self.octets.next().map(Octet::Hundreds))?
        {
            Octet::Hundreds(byte) => {
                self.current = Some(Octet::Tens(byte));
                byte / 100
            }
            Octet::Tens(byte) => {
                self.current = Some(Octet::Ones(byte));
                (byte % 100) / 10
            }
            Octet::Ones(byte) => {
                self.current = None;
                byte % 10
            }
        };
        Some(Digit::new(unsafe { RangedU8::new_unchecked(digit) }))
    }
}

#[cfg(test)]
mod tests {
    use crate::phone_number::{Digit, IntoDigits};
    use claims::{assert_none, assert_some_eq};
    use core::net::Ipv4Addr;
    use deranged::RangedU8;
    use gba_test::test;

    #[test]
    fn localhost_to_digits() {
        let mut digits = Ipv4Addr::LOCALHOST.into_digits();

        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<1>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<2>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<7>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<1>()));
        assert_none!(digits.next());
    }

    #[test]
    fn unspecified_to_digits() {
        let mut digits = Ipv4Addr::UNSPECIFIED.into_digits();

        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<0>()));
        assert_none!(digits.next());
    }

    #[test]
    fn broadcast_to_digits() {
        let mut digits = Ipv4Addr::BROADCAST.into_digits();

        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<2>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<5>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<5>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<2>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<5>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<5>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<2>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<5>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<5>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<2>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<5>()));
        assert_some_eq!(digits.next(), Digit::new(RangedU8::new_static::<5>()));
        assert_none!(digits.next());
    }
}
