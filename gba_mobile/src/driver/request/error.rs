use super::packet;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

/// Errors that can happen while sending or receiving a request.
#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Packet(packet::Error),
    NotIdle8(u8),
    NotIdle32(u32),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Packet(_) => formatter.write_str("packet communication error"),
            Self::NotIdle8(byte) => write!(
                formatter,
                "adapter did not respond with idle byte while no packet was being processed; received {byte:#04x}, expected 0xd2"
            ),
            Self::NotIdle32(byte) => write!(
                formatter,
                "adapter did not respond with idle bytes while no packet was being processed; received {byte:#010x}; expected 0xd2d2d2d2"
            ),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Packet(error) => Some(error),
            Self::NotIdle8(_) => None,
            Self::NotIdle32(_) => None,
        }
    }
}
