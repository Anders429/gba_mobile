use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
#[repr(u8)]
pub(in crate::engine) enum Error {
    FailedToDisconnect = 0x00,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::FailedToDisconnect => formatter.write_str("failed to reset adapter"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::FailedToDisconnect),
            _ => Err(UnknownError(byte)),
        }
    }
}
