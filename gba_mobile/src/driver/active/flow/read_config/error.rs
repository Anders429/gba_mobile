use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    ReadConfig(packet::Error<payload::ReadConfig>),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::ReadConfig(_) => formatter.write_str("error while reading config"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ReadConfig(error) => Some(error),
        }
    }
}
