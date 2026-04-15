use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    ReadConfig1(packet::Error<payload::ReadConfig>),
    ReadConfig2(packet::Error<payload::ReadConfig>),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
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
            Self::ReadConfig1(error) => Some(error),
            Self::ReadConfig2(error) => Some(error),
        }
    }
}
