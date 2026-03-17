use crate::driver::active::flow::request::packet;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    WriteConfig1(packet::Timeout),
    WriteConfig2(packet::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::WriteConfig1(_) => {
                formatter.write_str("timeout while writing first half of config")
            }
            Self::WriteConfig2(_) => {
                formatter.write_str("timeout while writing second half of config")
            }
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::WriteConfig1(timeout) => Some(timeout),
            Self::WriteConfig2(timeout) => Some(timeout),
        }
    }
}
