use crate::{arrayvec, driver, link};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub struct Error {
    kind: Kind,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl From<driver::error::link::Error> for Error {
    fn from(error: driver::error::link::Error) -> Self {
        Self {
            kind: Kind::Link(error.into()),
        }
    }
}

impl From<arrayvec::error::Capacity<32>> for Error {
    fn from(error: arrayvec::error::Capacity<32>) -> Self {
        Self {
            kind: Kind::PhoneNumber(error),
        }
    }
}

#[derive(Debug)]
enum Kind {
    Link(link::Error),
    PhoneNumber(arrayvec::error::Capacity<32>),
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Link(_) => formatter.write_str("Mobile Adapter link connection error"),
            Self::PhoneNumber(_) => formatter.write_str("phone number error"),
        }
    }
}

impl core::error::Error for Kind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Link(error) => Some(error),
            Self::PhoneNumber(error) => Some(error),
        }
    }
}
