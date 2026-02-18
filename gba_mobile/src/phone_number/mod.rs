pub mod digit;

pub use digit::Digit;

use crate::phone_number::digit::Pair;
use core::{
    fmt::{self, Debug, Display, Formatter},
    iter,
    net::Ipv4Addr,
    slice,
};
use deranged::RangedU8;

#[derive(Clone, Default, Eq, PartialEq)]
pub struct PhoneNumber([digit::Pair; 16]);

impl PhoneNumber {
    pub fn len(&self) -> u8 {
        self.into_iter().count() as u8
    }

    pub fn get(&self, index: u8) -> Option<Digit> {
        self.into_iter().nth(index as usize)
    }

    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        for digit in self.into_iter() {
            write!(formatter, "{digit}")?;
        }
        Ok(())
    }
}

impl Debug for PhoneNumber {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.fmt(formatter)
    }
}

impl Display for PhoneNumber {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.fmt(formatter)
    }
}

impl From<Ipv4Addr> for PhoneNumber {
    fn from(ip: Ipv4Addr) -> Self {
        ip.octets()
            .into_iter()
            .flat_map(|octet| [octet / 100, (octet % 100) / 10, octet % 10])
            .map(|digit| Digit::new(unsafe { RangedU8::new_unchecked(digit) }))
            .collect()
    }
}

impl FromIterator<Digit> for PhoneNumber {
    fn from_iter<T>(into_iter: T) -> Self
    where
        T: IntoIterator<Item = Digit>,
    {
        let mut iter = into_iter.into_iter();
        let mut pairs: [digit::Pair; 16] = Default::default();

        // We can take up to a maximum of 32 digits, handled 2 at a time.
        for i in 0..16 {
            let first = iter.next();
            // Only get a second digit if the first digit was `Some`.
            let second = first.and_then(|_| iter.next());

            pairs[i] = Pair::from_digits([first, second]);

            if first.is_none() || second.is_none() {
                // If either digit was `None`, we stop early.
                break;
            }
        }

        Self(pairs)
    }
}

impl<'a> IntoIterator for &'a PhoneNumber {
    type Item = Digit;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self.0.as_slice())
    }
}

pub struct Iter<'a> {
    digit_pairs: iter::Copied<slice::Iter<'a, digit::Pair>>,
    next_digit: Option<Option<Digit>>,
    terminated: bool,
}

impl<'a> Iter<'a> {
    fn new(digit_pairs: &'a [digit::Pair]) -> Self {
        Self {
            digit_pairs: digit_pairs.into_iter().copied(),
            next_digit: None,
            terminated: false,
        }
    }
}

impl Iterator for Iter<'_> {
    type Item = Digit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.terminated {
            None
        } else if let Some(digit) = self.next_digit.take() {
            if digit.is_none() {
                self.terminated = true;
            }
            digit
        } else if let Some(pair) = self.digit_pairs.next() {
            let [first, second] = pair.to_digits();
            if first.is_none() {
                self.terminated = true;
            } else {
                self.next_digit = Some(second);
            }
            first
        } else {
            self.terminated = true;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Digit, PhoneNumber};
    use alloc::format;
    use claims::{assert_none, assert_some_eq};
    use core::net::Ipv4Addr;
    use deranged::RangedU8;
    use gba_test::test;

    #[test]
    fn from_ipv4_localhost() {
        let phone_number: PhoneNumber = Ipv4Addr::LOCALHOST.into();
        assert_eq!(
            phone_number,
            [
                Digit::new(RangedU8::new_static::<1>()),
                Digit::new(RangedU8::new_static::<2>()),
                Digit::new(RangedU8::new_static::<7>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<1>()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn from_ipv4_unspecified() {
        let phone_number: PhoneNumber = Ipv4Addr::UNSPECIFIED.into();
        assert_eq!(
            phone_number,
            [
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
                Digit::new(RangedU8::new_static::<0>()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn from_ipv4_broadcast() {
        let phone_number: PhoneNumber = Ipv4Addr::BROADCAST.into();
        assert_eq!(
            phone_number,
            [
                Digit::new(RangedU8::new_static::<2>()),
                Digit::new(RangedU8::new_static::<5>()),
                Digit::new(RangedU8::new_static::<5>()),
                Digit::new(RangedU8::new_static::<2>()),
                Digit::new(RangedU8::new_static::<5>()),
                Digit::new(RangedU8::new_static::<5>()),
                Digit::new(RangedU8::new_static::<2>()),
                Digit::new(RangedU8::new_static::<5>()),
                Digit::new(RangedU8::new_static::<5>()),
                Digit::new(RangedU8::new_static::<2>()),
                Digit::new(RangedU8::new_static::<5>()),
                Digit::new(RangedU8::new_static::<5>()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn debug() {
        assert_eq!(
            format!(
                "{:?}",
                [
                    Digit::new(RangedU8::new_static::<8>()),
                    Digit::new(RangedU8::new_static::<6>()),
                    Digit::new(RangedU8::new_static::<7>()),
                    Digit::new(RangedU8::new_static::<5>()),
                    Digit::new(RangedU8::new_static::<3>()),
                    Digit::new(RangedU8::new_static::<0>()),
                    Digit::new(RangedU8::new_static::<9>()),
                ]
                .into_iter()
                .collect::<PhoneNumber>()
            ),
            "8675309"
        );
    }

    #[test]
    fn display() {
        assert_eq!(
            format!(
                "{}",
                [
                    Digit::new(RangedU8::new_static::<8>()),
                    Digit::new(RangedU8::new_static::<6>()),
                    Digit::new(RangedU8::new_static::<7>()),
                    Digit::new(RangedU8::new_static::<5>()),
                    Digit::new(RangedU8::new_static::<3>()),
                    Digit::new(RangedU8::new_static::<0>()),
                    Digit::new(RangedU8::new_static::<9>()),
                ]
                .into_iter()
                .collect::<PhoneNumber>()
            ),
            "8675309"
        );
    }

    #[test]
    fn iter() {
        let phone_number = [
            Digit::new(RangedU8::new_static::<1>()),
            Digit::new(RangedU8::new_static::<2>()),
            Digit::new(RangedU8::new_static::<3>()),
            Digit::new(RangedU8::new_static::<4>()),
        ]
        .into_iter()
        .collect::<PhoneNumber>();
        let mut iter = phone_number.into_iter();

        assert_some_eq!(iter.next(), Digit::new(RangedU8::new_static::<1>()));
        assert_some_eq!(iter.next(), Digit::new(RangedU8::new_static::<2>()));
        assert_some_eq!(iter.next(), Digit::new(RangedU8::new_static::<3>()));
        assert_some_eq!(iter.next(), Digit::new(RangedU8::new_static::<4>()));
        assert_none!(iter.next());
    }
}
