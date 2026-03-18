use crate::driver::active::flow::request::packet;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    ConnectionStatus(packet::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::ConnectionStatus(_) => {
                formatter.write_str("timeout while checking connection status")
            }
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ConnectionStatus(timeout) => Some(timeout),
        }
    }
}
