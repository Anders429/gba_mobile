mod pair;

pub(crate) use pair::Pair;

use core::{
    fmt::{self, Debug, Display, Formatter, Write},
    hint::unreachable_unchecked,
};
use deranged::RangedU8;

#[derive(Clone, Copy, Debug)]
pub struct Invalid<T>(pub T);

impl<T> Display for Invalid<T>
where
    T: Display,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{} is not a valid digit", self.0)
    }
}

impl<T> core::error::Error for Invalid<T> where T: Debug + Display {}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Digit(RangedU8<0, 11>);

impl Digit {
    /// Create a new digit using a raw value.
    ///
    /// Values are mapped as follows:
    /// - `0-9`: digits `0-9`.
    /// - `10`: `#`
    /// - `11`: `*`.
    pub fn new(value: RangedU8<0, 11>) -> Self {
        Self(value)
    }

    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_char((*self).into())
    }
}

impl Debug for Digit {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.fmt(formatter)
    }
}

impl Display for Digit {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.fmt(formatter)
    }
}

impl TryFrom<char> for Digit {
    type Error = Invalid<char>;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            '0' => Ok(Self(RangedU8::new_static::<0x0>())),
            '1' => Ok(Self(RangedU8::new_static::<0x1>())),
            '2' => Ok(Self(RangedU8::new_static::<0x2>())),
            '3' => Ok(Self(RangedU8::new_static::<0x3>())),
            '4' => Ok(Self(RangedU8::new_static::<0x4>())),
            '5' => Ok(Self(RangedU8::new_static::<0x5>())),
            '6' => Ok(Self(RangedU8::new_static::<0x6>())),
            '7' => Ok(Self(RangedU8::new_static::<0x7>())),
            '8' => Ok(Self(RangedU8::new_static::<0x8>())),
            '9' => Ok(Self(RangedU8::new_static::<0x9>())),
            '#' => Ok(Self(RangedU8::new_static::<0xa>())),
            '*' => Ok(Self(RangedU8::new_static::<0xb>())),
            invalid => Err(Invalid(invalid)),
        }
    }
}

impl TryFrom<u8> for Digit {
    type Error = Invalid<u8>;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            b'0' => Ok(Self(RangedU8::new_static::<0x0>())),
            b'1' => Ok(Self(RangedU8::new_static::<0x1>())),
            b'2' => Ok(Self(RangedU8::new_static::<0x2>())),
            b'3' => Ok(Self(RangedU8::new_static::<0x3>())),
            b'4' => Ok(Self(RangedU8::new_static::<0x4>())),
            b'5' => Ok(Self(RangedU8::new_static::<0x5>())),
            b'6' => Ok(Self(RangedU8::new_static::<0x6>())),
            b'7' => Ok(Self(RangedU8::new_static::<0x7>())),
            b'8' => Ok(Self(RangedU8::new_static::<0x8>())),
            b'9' => Ok(Self(RangedU8::new_static::<0x9>())),
            b'#' => Ok(Self(RangedU8::new_static::<0xa>())),
            b'*' => Ok(Self(RangedU8::new_static::<0xb>())),
            invalid => Err(Invalid(invalid)),
        }
    }
}

impl From<Digit> for char {
    fn from(digit: Digit) -> Self {
        match digit.0.get() {
            0x0 => '0',
            0x1 => '1',
            0x2 => '2',
            0x3 => '3',
            0x4 => '4',
            0x5 => '5',
            0x6 => '6',
            0x7 => '7',
            0x8 => '8',
            0x9 => '9',
            0xa => '#',
            0xb => '*',
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

impl From<Digit> for u8 {
    fn from(digit: Digit) -> Self {
        match digit.0.get() {
            0x0 => b'0',
            0x1 => b'1',
            0x2 => b'2',
            0x3 => b'3',
            0x4 => b'4',
            0x5 => b'5',
            0x6 => b'6',
            0x7 => b'7',
            0x8 => b'8',
            0x9 => b'9',
            0xa => b'#',
            0xb => b'*',
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Digit;
    use alloc::format;
    use claims::assert_ok;
    use gba_test::test;

    #[test]
    fn digit_display() {
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('0'))), "0");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('1'))), "1");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('2'))), "2");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('3'))), "3");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('4'))), "4");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('5'))), "5");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('6'))), "6");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('7'))), "7");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('8'))), "8");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('9'))), "9");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('#'))), "#");
        assert_eq!(format!("{}", assert_ok!(Digit::try_from('*'))), "*");
    }

    #[test]
    fn digit_debug() {
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('0'))), "0");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('1'))), "1");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('2'))), "2");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('3'))), "3");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('4'))), "4");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('5'))), "5");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('6'))), "6");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('7'))), "7");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('8'))), "8");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('9'))), "9");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('#'))), "#");
        assert_eq!(format!("{:?}", assert_ok!(Digit::try_from('*'))), "*");
    }
}
