use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    ConnectionStatus(packet::Error<payload::ConnectionStatus>),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::ConnectionStatus(_) => {
                formatter.write_str("error while checking connection status")
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ConnectionStatus(error) => Some(error),
        }
    }
}
