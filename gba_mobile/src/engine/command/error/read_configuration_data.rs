use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
#[repr(u8)]
pub(in crate::engine) enum Error {
    ReadFailure = 0x00,
    InvalidParameters = 0x02,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::ReadFailure => formatter.write_str("failed to read configuration data"),
            Self::InvalidParameters => formatter.write_str("invalid read parameters"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::ReadFailure),
            0x02 => Ok(Self::InvalidParameters),
            _ => Err(UnknownError(byte)),
        }
    }
}
