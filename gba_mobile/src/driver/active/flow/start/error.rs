use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    BeginSession(packet::Error<payload::BeginSession>),
    Sio32(packet::Error<payload::EnableSio32>),
    ReadConfig1(packet::Error<payload::ReadConfig>),
    ReadConfig2(packet::Error<payload::ReadConfig>),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::BeginSession(_) => formatter.write_str("error while beginning session"),
            Self::Sio32(_) => formatter.write_str("error while enabling SIO32 mode"),
            Self::ReadConfig1(_) => formatter.write_str("error while reading first half of config"),
            Self::ReadConfig2(_) => {
                formatter.write_str("error while reading second half of config")
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::BeginSession(error) => Some(error),
            Self::Sio32(error) => Some(error),
            Self::ReadConfig1(error) => Some(error),
            Self::ReadConfig2(error) => Some(error),
        }
    }
}
