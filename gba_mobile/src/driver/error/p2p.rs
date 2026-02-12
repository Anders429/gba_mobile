use core::{
    fmt,
    fmt::{Display, Formatter},
};

use crate::driver::{command, error::link};

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

impl From<link::Error> for Error {
    fn from(error: link::Error) -> Self {
        Self {
            kind: Kind::Link(error),
        }
    }
}

impl From<command::Error> for Error {
    fn from(error: command::Error) -> Self {
        Self {
            kind: Kind::Command(error),
        }
    }
}

#[derive(Debug)]
enum Kind {
    Command(command::Error),
    Closed,
    Superseded,

    Link(link::Error),
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Command(_) => formatter.write_str("the adapter responded with an error"),
            Self::Closed => formatter.write_str("the connection was closed"),
            Self::Superseded => formatter.write_str("the connection attempt was superseded"),

            Self::Link(_) => formatter.write_str("link connection error"),
        }
    }
}

impl core::error::Error for Kind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Command(error) => Some(error),
            Self::Closed => None,
            Self::Superseded => None,

            Self::Link(error) => Some(error),
        }
    }
}
