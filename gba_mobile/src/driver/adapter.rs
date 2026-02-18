use core::{
    fmt,
    fmt::{Display, Formatter},
};

/// An unknown adapter ID that does not correspond to a supported adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::driver) struct Unknown(u8);

impl Display for Unknown {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "unknown adapter ID: {:#04x}", self.0)
    }
}

impl core::error::Error for Unknown {}

/// The type of adapter being used.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::driver) enum Adapter {
    /// PDC Mobile Adapter (Blue).
    Blue = 0x88,
    /// cdmaOne Mobile Adapter (Yellow).
    Yellow = 0x89,
    /// PHS Mobile Adapter (Green).
    Green = 0x8a,
    /// DDI Mobile Adapter (Red).
    Red = 0x8b,
}

impl Adapter {
    /// The byte used when dialing a number.
    ///
    /// The required byte is different depending on the adapter being used.
    pub(in crate::driver) fn dial_byte(self) -> u8 {
        match self {
            Self::Blue => 0,
            Self::Yellow => 2,
            Self::Green => 1,
            Self::Red => 1,
        }
    }
}

impl TryFrom<u8> for Adapter {
    type Error = Unknown;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x88 => Ok(Self::Blue),
            0x89 => Ok(Self::Yellow),
            0x8a => Ok(Self::Green),
            0x8b => Ok(Self::Red),
            unknown => Err(Unknown(unknown)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Adapter, Unknown};
    use alloc::format;
    use claims::{assert_err_eq, assert_ok_eq};
    use gba_test::test;

    #[test]
    fn dial_byte_blue() {
        assert_eq!(Adapter::Blue.dial_byte(), 0);
    }

    #[test]
    fn dial_byte_yellow() {
        assert_eq!(Adapter::Yellow.dial_byte(), 2);
    }

    #[test]
    fn dial_byte_green() {
        assert_eq!(Adapter::Green.dial_byte(), 1);
    }

    #[test]
    fn dial_byte_red() {
        assert_eq!(Adapter::Red.dial_byte(), 1);
    }

    #[test]
    fn try_from_blue() {
        assert_ok_eq!(Adapter::try_from(0x88), Adapter::Blue);
    }

    #[test]
    fn try_from_yellow() {
        assert_ok_eq!(Adapter::try_from(0x89), Adapter::Yellow);
    }

    #[test]
    fn try_from_green() {
        assert_ok_eq!(Adapter::try_from(0x8a), Adapter::Green);
    }

    #[test]
    fn try_from_red() {
        assert_ok_eq!(Adapter::try_from(0x8b), Adapter::Red);
    }

    #[test]
    fn try_from_unknown() {
        assert_err_eq!(Adapter::try_from(0xff), Unknown(0xff));
    }

    #[test]
    fn unknown_display_ff() {
        assert_eq!(format!("{}", Unknown(0xff)), "unknown adapter ID: 0xff");
    }

    #[test]
    fn unknown_display_00() {
        // Make sure that we aren't cutting off 0s.
        assert_eq!(format!("{}", Unknown(0x00)), "unknown adapter ID: 0x00");
    }
}
