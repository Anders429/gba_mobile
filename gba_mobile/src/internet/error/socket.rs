use crate::{arrayvec, driver, internet};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

#[derive(Debug)]
pub struct Error<SocketError> {
    kind: Kind<SocketError>,
}

impl<SocketError> Error<SocketError> {
    pub(crate) fn socket(error: SocketError) -> Self {
        Self {
            kind: Kind::Socket(error),
        }
    }
}

impl<SocketError> Display for Error<SocketError> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl<SocketError> core::error::Error for Error<SocketError>
where
    SocketError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<SocketError> From<driver::error::connection::Error> for Error<SocketError> {
    fn from(error: driver::error::connection::Error) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

impl<SocketError> From<arrayvec::error::Capacity<255>> for Error<SocketError> {
    fn from(error: arrayvec::error::Capacity<255>) -> Self {
        Self {
            kind: Kind::DomainNameCapacity(error),
        }
    }
}

#[derive(Debug)]
enum Kind<SocketError> {
    Connection(internet::Error),
    Socket(SocketError),
    DomainNameCapacity(arrayvec::error::Capacity<255>),
}

impl<SocketError> Display for Kind<SocketError> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Connection(_) => formatter.write_str("Mobile Adapter connection error"),
            Self::Socket(_) => formatter.write_str("failed to convert to socket address"),
            Self::DomainNameCapacity(_) => {
                formatter.write_str("could not process domain name lookup request")
            }
        }
    }
}

impl<SocketError> core::error::Error for Kind<SocketError>
where
    SocketError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Connection(error) => Some(error),
            Self::Socket(error) => Some(error),
            Self::DomainNameCapacity(error) => Some(error),
        }
    }
}
