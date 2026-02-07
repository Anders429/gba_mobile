use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::driver) enum Error {
    CommunicationFailed = 0x00,
    NotConnected = 0x01,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::CommunicationFailed => formatter.write_str("failed to transfer data"),
            Self::NotConnected => formatter.write_str("no connection"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::CommunicationFailed),
            0x01 => Ok(Self::NotConnected),
            _ => Err(UnknownError(byte)),
        }
    }
}
