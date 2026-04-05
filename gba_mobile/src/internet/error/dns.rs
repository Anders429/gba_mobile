use crate::{arrayvec, driver, internet};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

#[derive(Debug)]
pub struct Error<const MAX_LEN: usize> {
    kind: Kind<MAX_LEN>,
}

impl<const MAX_LEN: usize> Display for Error<MAX_LEN> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.kind, formatter)
    }
}

impl<const MAX_LEN: usize> core::error::Error for Error<MAX_LEN> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<const MAX_LEN: usize> From<driver::error::connection::Error> for Error<MAX_LEN> {
    fn from(error: driver::error::connection::Error) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

impl<const MAX_LEN: usize> From<arrayvec::error::Capacity<MAX_LEN>> for Error<MAX_LEN> {
    fn from(error: arrayvec::error::Capacity<MAX_LEN>) -> Self {
        Self {
            kind: Kind::Capacity(error),
        }
    }
}

#[derive(Debug)]
enum Kind<const MAX_LEN: usize> {
    Capacity(arrayvec::error::Capacity<MAX_LEN>),
    Connection(internet::Error),
}

impl<const MAX_LEN: usize> Display for Kind<MAX_LEN> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Capacity(_) => formatter.write_str("could not create domain name lookup request"),
            Self::Connection(_) => formatter.write_str("Mobile Adapter connection error"),
        }
    }
}

impl<const MAX_LEN: usize> core::error::Error for Kind<MAX_LEN> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Capacity(error) => Some(error),
            Self::Connection(error) => Some(error),
        }
    }
}
