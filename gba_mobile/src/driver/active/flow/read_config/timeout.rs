use crate::driver::active::flow::request::packet;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    ReadConfig1(packet::Timeout),
    ReadConfig2(packet::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::ReadConfig1(_) => {
                formatter.write_str("timeout while reading first half of config")
            }
            Self::ReadConfig2(_) => {
                formatter.write_str("timeout while reading second half of config")
            }
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ReadConfig1(timeout) => Some(timeout),
            Self::ReadConfig2(timeout) => Some(timeout),
        }
    }
}
