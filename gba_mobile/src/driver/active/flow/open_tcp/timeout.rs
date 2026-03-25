use crate::driver::active::flow::request::packet;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    Dns(packet::Timeout),
    OpenTcp(packet::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Dns(_) => formatter.write_str("timeout while querying DNS"),
            Self::OpenTcp(_) => formatter.write_str("timeout while opening TCP connection"),
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Dns(timeout) => Some(timeout),
            Self::OpenTcp(timeout) => Some(timeout),
        }
    }
}
