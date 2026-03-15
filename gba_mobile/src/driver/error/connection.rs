use super::link;
use crate::driver::active::ConnectionFailure;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(crate) struct Error {
    kind: Kind,
}

impl Error {
    pub(in crate::driver) fn closed() -> Self {
        Self { kind: Kind::Closed }
    }

    pub(in crate::driver) fn superseded() -> Self {
        Self {
            kind: Kind::Superseded,
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

impl From<ConnectionFailure> for Error {
    fn from(error: ConnectionFailure) -> Self {
        Self {
            kind: Kind::Failure(error),
        }
    }
}

impl From<link::Error> for Error {
    fn from(error: link::Error) -> Self {
        Self {
            kind: Kind::Link(error),
        }
    }
}

#[derive(Debug)]
enum Kind {
    Closed,
    Superseded,
    Failure(ConnectionFailure),
    Link(link::Error),
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("the connection was closed"),
            Self::Superseded => formatter.write_str("the connection was superseded"),
            Self::Failure(_) => formatter.write_str("failed to establish connection"),
            Self::Link(_) => formatter.write_str("link error"),
        }
    }
}

impl core::error::Error for Kind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Closed => None,
            Self::Superseded => None,
            Self::Failure(error) => Some(error),
            Self::Link(error) => Some(error),
        }
    }
}
