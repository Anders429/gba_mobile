use crate::driver::active::flow::request::{packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Connect(packet::Error<payload::Connect>),
    Login(packet::Error<payload::Login>),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Connect(_) => formatter.write_str("error while connecting"),
            Self::Login(_) => formatter.write_str("error while logging in"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Connect(error) => Some(error),
            Self::Login(error) => Some(error),
        }
    }
}
