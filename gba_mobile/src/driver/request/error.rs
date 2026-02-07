use super::packet;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

/// Errors that can happen while sending or receiving a request.
#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Packet(packet::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Packet(_) => formatter.write_str("packet communication error"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Packet(error) => Some(error),
        }
    }
}
