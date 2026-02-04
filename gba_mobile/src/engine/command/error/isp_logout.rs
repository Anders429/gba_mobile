use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::engine) enum Error {
    NotLoggedIn = 0x00,
    NotInCall = 0x01,
    Timeout = 0x02,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::NotLoggedIn => formatter.write_str("not logged in"),
            Self::NotInCall => formatter.write_str("not in a call"),
            Self::Timeout => formatter.write_str("timeout"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::NotLoggedIn),
            0x01 => Ok(Self::NotInCall),
            0x02 => Ok(Self::Timeout),
            _ => Err(UnknownError(byte)),
        }
    }
}
