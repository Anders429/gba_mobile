use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    BeginSession(packet::Error<payload::BeginSession>),
    Sio32(packet::Error<payload::EnableSio32>),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::BeginSession(_) => formatter.write_str("error while beginning session"),
            Self::Sio32(_) => formatter.write_str("error while enabling SIO32 mode"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::BeginSession(error) => Some(error),
            Self::Sio32(error) => Some(error),
        }
    }
}
