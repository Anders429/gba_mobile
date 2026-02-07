use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::driver) enum Error {
    NoCallReceived = 0x00,
    AlreadyCalling = 0x01,
    InternalError = 0x03,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::NoCallReceived => formatter.write_str("no call received"),
            Self::AlreadyCalling => formatter.write_str("already calling"),
            Self::InternalError => formatter.write_str("adapter internal error"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::NoCallReceived),
            0x01 => Ok(Self::AlreadyCalling),
            0x03 => Ok(Self::InternalError),
            _ => Err(UnknownError(byte)),
        }
    }
}
