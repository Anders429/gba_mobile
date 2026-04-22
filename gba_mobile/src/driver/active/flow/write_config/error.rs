use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    WriteConfig(packet::Error<payload::WriteConfig>),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::WriteConfig(_) => formatter.write_str("error while writing config"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::WriteConfig(error) => Some(error),
        }
    }
}
