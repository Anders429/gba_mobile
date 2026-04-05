use super::{connection, link};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(crate) struct Error {
    kind: Kind,
}

impl Error {
    pub(in crate::driver) fn superseded() -> Self {
        Self {
            kind: Kind::Superseded,
        }
    }

    pub(in crate::driver) fn not_found() -> Self {
        Self {
            kind: Kind::NotFound,
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
    Superseded,
    NotFound,
    Connection(connection::Error),
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Superseded => formatter.write_str("the DNS request was superseded"),
            Self::NotFound => formatter.write_str("domain lookup failed"),
            Self::Connection(_) => formatter.write_str("connection error"),
        }
    }
}

impl core::error::Error for Kind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Superseded => None,
            Self::NotFound => None,
            Self::Connection(error) => Some(error),
        }
    }
}
