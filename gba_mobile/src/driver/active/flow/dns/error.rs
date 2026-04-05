use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error<const MAX_LEN: usize> {
    Dns(packet::Error<payload::Dns<MAX_LEN>>),
}

impl<const MAX_LEN: usize> Display for Error<MAX_LEN> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Dns(_) => formatter.write_str("error while querying DNS"),
        }
    }
}

impl<const MAX_LEN: usize> core::error::Error for Error<MAX_LEN> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Dns(error) => Some(error),
        }
    }
}
