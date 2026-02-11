use crate::driver::{command, request};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(crate) struct Error {
    kind: Kind,
}

impl Error {
    pub(in crate::driver) fn aborted() -> Self {
        Self {
            kind: Kind::Aborted,
        }
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

impl From<request::Error> for Error {
    fn from(error: request::Error) -> Self {
        Self {
            kind: Kind::Request(error),
        }
    }
}

impl From<request::Timeout> for Error {
    fn from(timeout: request::Timeout) -> Self {
        Self {
            kind: Kind::Timeout(timeout),
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
    Request(request::Error),
    Timeout(request::Timeout),
    Command(command::Error),
    Aborted,
    Superseded,
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Request(_) => {
                formatter.write_str("an error occurred while processing the request")
            }
            Self::Timeout(_) => formatter.write_str("the request timed out"),
            Self::Command(_) => formatter.write_str("the adapter responded with an error"),
            Self::Aborted => formatter.write_str("the link attempt was aborted"),
            Self::Superseded => formatter.write_str("the link attempt was superseded"),
        }
    }
}

impl core::error::Error for Kind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            Self::Timeout(timeout) => Some(timeout),
            Self::Command(error) => Some(error),
            Self::Aborted => None,
            Self::Superseded => None,
        }
    }
}
