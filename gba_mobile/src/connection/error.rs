use crate::driver;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub struct P2p {
    internal: driver::error::connection::Error,
}

impl Display for P2p {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.internal.fmt(formatter)
    }
}

impl core::error::Error for P2p {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl From<driver::error::connection::Error> for P2p {
    fn from(error: driver::error::connection::Error) -> Self {
        Self { internal: error }
    }
}

#[derive(Debug)]
pub struct Socket {
    internal: driver::error::socket::Error,
}

impl Display for Socket {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.internal.fmt(formatter)
    }
}

impl core::error::Error for Socket {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl From<driver::error::socket::Error> for Socket {
    fn from(error: driver::error::socket::Error) -> Self {
        Self { internal: error }
    }
}
