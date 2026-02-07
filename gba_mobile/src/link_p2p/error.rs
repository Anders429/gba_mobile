use crate::driver;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub struct Error {
    internal: driver::error::link_p2p::Error,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.internal.fmt(formatter)
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl From<driver::error::link_p2p::Error> for Error {
    fn from(error: driver::error::link_p2p::Error) -> Self {
        Self { internal: error }
    }
}
