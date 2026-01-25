use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
#[repr(u8)]
pub(in crate::engine) enum Error {
    AlreadyActive = 0x01,
    InvalidContents = 0x02,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::AlreadyActive => formatter.write_str("session already active"),
            Self::InvalidContents => formatter.write_str("incorrect payload contents"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x01 => Ok(Self::AlreadyActive),
            0x02 => Ok(Self::InvalidContents),
            _ => Err(UnknownError(byte)),
        }
    }
}
