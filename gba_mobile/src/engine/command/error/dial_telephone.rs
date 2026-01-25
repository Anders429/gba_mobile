use super::UnknownError;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
#[repr(u8)]
pub(in crate::engine) enum Error {
    LineBusy = 0x00,
    AlreadyConnected = 0x01,
    InvalidContents = 0x02,
    CommunicationFailed = 0x03,
    CallNotEstablished = 0x04,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::LineBusy => formatter.write_str("the phone line is busy"),
            Self::AlreadyConnected => formatter.write_str("a call is already connected"),
            Self::InvalidContents => formatter.write_str("a call is already connected"),
            Self::CommunicationFailed => formatter.write_str("could not connect"),
            Self::CallNotEstablished => formatter.write_str("call not established"),
        }
    }
}

impl core::error::Error for Error {}

impl TryFrom<u8> for Error {
    type Error = UnknownError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::LineBusy),
            0x01 => Ok(Self::AlreadyConnected),
            0x02 => Ok(Self::InvalidContents),
            0x03 => Ok(Self::CommunicationFailed),
            0x04 => Ok(Self::CallNotEstablished),
            _ => Err(UnknownError(byte)),
        }
    }
}
