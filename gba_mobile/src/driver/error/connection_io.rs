use super::{connection, link};
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

impl<IoError> From<connection::Error> for Error<IoError> {
    fn from(error: connection::Error) -> Self {
        Self {
            kind: Kind::Connection(error),
        }
    }
}

impl<IoError> From<link::Error> for Error<IoError> {
    fn from(error: link::Error) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

#[derive(Debug)]
enum Kind<IoError> {
    Io(IoError),
    Connection(connection::Error),
}

impl<IoError> Display for Kind<IoError>
where
    IoError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_) => formatter.write_str("io error"),
            Self::Connection(_) => formatter.write_str("connection error"),
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
            Self::Connection(error) => Some(error),
        }
    }
}
