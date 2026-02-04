use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::engine) enum Error {
    InvalidContents = 0x02,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::InvalidContents => {
                formatter.write_str("invalid contents; expected to receive a `0` or a `1`")
            }
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x02 => Ok(Self::InvalidContents),
            _ => Err(UnknownError(byte)),
        }
    }
}
