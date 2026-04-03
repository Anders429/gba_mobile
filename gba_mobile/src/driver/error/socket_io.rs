use super::{connection, link, socket};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(crate) struct Error<IoError> {
    kind: Kind<IoError>,
}

impl<IoError> Error<IoError> {
    pub(in crate::driver) fn io(error: IoError) -> Self {
        Self {
            kind: Kind::Io(error),
        }
    }
}

impl<IoError> Display for Error<IoError>
where
    IoError: Display,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl<IoError> core::error::Error for Error<IoError>
where
    IoError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<IoError> From<socket::Error> for Error<IoError> {
    fn from(error: socket::Error) -> Self {
        Self {
            kind: Kind::Socket(error),
        }
    }
}

impl<IoError> From<connection::Error> for Error<IoError> {
    fn from(error: connection::Error) -> Self {
        Self {
            kind: Kind::Socket(error.into()),
        }
    }
}

impl<IoError> From<link::Error> for Error<IoError> {
    fn from(error: link::Error) -> Self {
        Self {
            kind: Kind::Socket(error.into()),
        }
    }
}

#[derive(Debug)]
enum Kind<IoError> {
    Io(IoError),
    Socket(socket::Error),
}

impl<IoError> Display for Kind<IoError>
where
    IoError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_) => formatter.write_str("io error"),
            Self::Socket(_) => formatter.write_str("socket error"),
        }
    }
}

impl<IoError> core::error::Error for Kind<IoError>
where
    IoError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Socket(error) => Some(error),
        }
    }
}
