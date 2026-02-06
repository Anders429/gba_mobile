use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::engine) enum Error {
    NotConnected = 0x00,
    NotLoggedIn = 0x01,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::NotConnected => formatter.write_str("not connected"),
            Self::NotLoggedIn => formatter.write_str("not logged in"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::NotConnected),
            0x01 => Ok(Self::NotLoggedIn),
            _ => Err(UnknownError(byte)),
        }
    }
}
