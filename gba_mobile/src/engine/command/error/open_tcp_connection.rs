use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::engine) enum Error {
    TooManyConnections = 0x00,
    NotLoggedIn = 0x01,
    ConnectionFailed = 0x03,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::TooManyConnections => formatter.write_str("too many connections"),
            Self::NotLoggedIn => formatter.write_str("not logged in"),
            Self::ConnectionFailed => formatter.write_str("failed to open connection"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::TooManyConnections),
            0x01 => Ok(Self::NotLoggedIn),
            0x03 => Ok(Self::ConnectionFailed),
            _ => Err(UnknownError(byte)),
        }
    }
}
