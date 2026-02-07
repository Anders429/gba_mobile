use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::driver) enum Error {
    WriteFailure = 0x00,
    InvalidParameters = 0x02,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::WriteFailure => formatter.write_str("failed to write configuration data"),
            Self::InvalidParameters => formatter.write_str("invalid write parameters"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::WriteFailure),
            0x02 => Ok(Self::InvalidParameters),
            _ => Err(UnknownError(byte)),
        }
    }
}
