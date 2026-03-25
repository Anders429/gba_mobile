use super::{connection, link};
use crate::driver::active::socket;
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

impl From<socket::Failure> for Error {
    fn from(error: socket::Failure) -> Self {
        Self {
            kind: Kind::Failure(error),
        }
    }
}

impl From<connection::Error> for Error {
    fn from(error: connection::Error) -> Self {
        Self {
            kind: Kind::Connection(error),
        }
    }
}

impl From<link::Error> for Error {
    fn from(error: link::Error) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

#[derive(Debug)]
enum Kind {
    Closed,
    Superseded,
    Failure(socket::Failure),
    Connection(connection::Error),
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("the socket was closed"),
            Self::Superseded => formatter.write_str("the socket connection was superseded"),
            Self::Failure(_) => formatter.write_str("failed to connect socket"),
            Self::Connection(_) => formatter.write_str("connection error"),
        }
    }
}

impl core::error::Error for Kind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Closed => None,
            Self::Superseded => None,
            Self::Failure(error) => Some(error),
            Self::Connection(error) => Some(error),
        }
    }
}
