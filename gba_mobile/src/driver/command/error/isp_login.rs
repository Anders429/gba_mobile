use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::driver) enum Error {
    NotInCall = 0x01,
    Timeout = 0x02,
    InternalError = 0x03,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::NotInCall => formatter.write_str("not in a call"),
            Self::Timeout => formatter.write_str("timeout"),
            Self::InternalError => formatter.write_str("adapter internal error"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x01 => Ok(Self::NotInCall),
            0x02 => Ok(Self::Timeout),
            0x03 => Ok(Self::InternalError),
            _ => Err(UnknownError(byte)),
        }
    }
}
