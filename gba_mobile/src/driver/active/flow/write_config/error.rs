use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    WriteConfig1(packet::Error<payload::WriteConfig>),
    WriteConfig2(packet::Error<payload::WriteConfig>),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::WriteConfig1(_) => {
                formatter.write_str("error while writing first half of config")
            }
            Self::WriteConfig2(_) => {
                formatter.write_str("error while writing second half of config")
            }
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::WriteConfig1(error) => Some(error),
            Self::WriteConfig2(error) => Some(error),
        }
    }
}
