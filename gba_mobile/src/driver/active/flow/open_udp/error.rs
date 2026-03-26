use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Dns(packet::Error<payload::Dns>),
    OpenUdp(packet::Error<payload::OpenUdp>),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Dns(_) => formatter.write_str("error while querying DNS"),
            Self::OpenUdp(_) => formatter.write_str("error while opening UDP connection"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Dns(error) => Some(error),
            Self::OpenUdp(error) => Some(error),
        }
    }
}
