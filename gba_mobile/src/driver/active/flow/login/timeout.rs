use crate::driver::active::flow::request::packet;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    Connect(packet::Timeout),
    Login(packet::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Connect(_) => formatter.write_str("timeout while connecting"),
            Self::Login(_) => formatter.write_str("timeout while logging in"),
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Connect(timeout) => Some(timeout),
            Self::Login(timeout) => Some(timeout),
        }
    }
}
