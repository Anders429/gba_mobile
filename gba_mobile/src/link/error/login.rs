use super::connect;
use crate::{arrayvec, driver};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub struct Error {
    kind: Kind,
}

impl Error {
    pub(crate) fn id(error: arrayvec::error::Capacity<32>) -> Self {
        Self {
            kind: Kind::Id(error),
        }
    }

    pub(crate) fn password(error: arrayvec::error::Capacity<32>) -> Self {
        Self {
            kind: Kind::Password(error),
        }
    }
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
            kind: Kind::Connect(error.into()),
        }
    }
}

impl From<arrayvec::error::Capacity<32>> for Error {
    fn from(error: arrayvec::error::Capacity<32>) -> Self {
        Self {
            kind: Kind::Connect(error.into()),
        }
    }
}

#[derive(Debug)]
enum Kind {
    Connect(connect::Error),
    Id(arrayvec::error::Capacity<32>),
    Password(arrayvec::error::Capacity<32>),
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Connect(_) => formatter.write_str("connection error"),
            Self::Id(_) => formatter.write_str("ID too long"),
            Self::Password(_) => formatter.write_str("password too long"),
        }
    }
}

impl core::error::Error for Kind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Connect(error) => Some(error),
            Self::Id(error) => Some(error),
            Self::Password(error) => Some(error),
        }
    }
}
