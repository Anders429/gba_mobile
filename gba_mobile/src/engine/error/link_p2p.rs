use crate::engine::{command, request};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(crate) struct Error {
    kind: Kind,
}

impl Error {
    pub(in crate::engine) fn aborted() -> Self {
        Self {
            kind: Kind::Aborted,
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
    Command(command::Error),
    Aborted,
}

impl Display for Kind {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Request(_) => {
                formatter.write_str("an error occurred while processing the request")
            }
            Self::Command(_) => formatter.write_str("the adapter responded with an error"),
            Self::Aborted => formatter.write_str("the link attempt was aborted"),
        }
    }
}

impl core::error::Error for Kind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            Self::Command(error) => Some(error),
            Self::Aborted => None,
        }
    }
}
