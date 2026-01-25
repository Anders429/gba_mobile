use super::{receive, send};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

/// Errors that can happen while sending or receiving a packet.
#[derive(Debug)]
pub(in crate::engine) enum Error {
    Send(send::Error),
    Receive(receive::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Send(_) => formatter.write_str("error during packet sending"),
            Self::Receive(_) => formatter.write_str("error during packet receiving"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Send(error) => Some(error),
            Self::Receive(error) => Some(error),
        }
    }
}
