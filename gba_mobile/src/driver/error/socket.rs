use super::{connection, link};
use crate::driver::active::ConnectionFailure;
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub(crate) struct Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    kind: Kind<Socket1, Socket2, Dns>,
}

impl<Socket1, Socket2, Dns> Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    pub(in crate::driver) fn closed() -> Self {
        Self { kind: Kind::Closed }
    }

    pub(in crate::driver) fn superseded() -> Self {
        Self {
            kind: Kind::Superseded,
        }
    }

    pub(in crate::driver) fn failed_to_connect() -> Self {
        Self {
            kind: Kind::FailedToConnect,
        }
    }

    pub(in crate::driver) fn closed_remotely() -> Self {
        Self {
            kind: Kind::ClosedRemotely,
        }
    }
}

impl<Socket1, Socket2, Dns> Debug for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.kind, formatter)
    }
}

impl<Socket1, Socket2, Dns> Display for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.kind, formatter)
    }
}

impl<Socket1, Socket2, Dns> core::error::Error for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<Socket1, Socket2, Dns> From<connection::Error<Socket1, Socket2, Dns>>
    for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn from(error: connection::Error<Socket1, Socket2, Dns>) -> Self {
        Self {
            kind: Kind::Connection(error),
        }
    }
}

impl<Socket1, Socket2, Dns> From<ConnectionFailure> for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn from(error: ConnectionFailure) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

impl<Socket1, Socket2, Dns> From<link::Error<Socket1, Socket2, Dns>>
    for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn from(error: link::Error<Socket1, Socket2, Dns>) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

enum Kind<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    Closed,
    Superseded,
    FailedToConnect,
    ClosedRemotely,
    Connection(connection::Error<Socket1, Socket2, Dns>),
}

impl<Socket1, Socket2, Dns> Debug for Kind<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("Closed"),
            Self::Superseded => formatter.write_str("Superseded"),
            Self::FailedToConnect => formatter.write_str("FailedToConnect"),
            Self::ClosedRemotely => formatter.write_str("ClosedRemotely"),
            Self::Connection(error) => formatter.debug_tuple("Connection").field(error).finish(),
        }
    }
}

impl<Socket1, Socket2, Dns> Display for Kind<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("the socket was closed"),
            Self::Superseded => formatter.write_str("the socket connection was superseded"),
            Self::FailedToConnect => {
                formatter.write_str("the socket connection could not be established")
            }
            Self::ClosedRemotely => formatter.write_str("the socket was closed by the remote host"),
            Self::Connection(_) => formatter.write_str("connection error"),
        }
    }
}

impl<Socket1, Socket2, Dns> core::error::Error for Kind<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Closed => None,
            Self::Superseded => None,
            Self::FailedToConnect => None,
            Self::ClosedRemotely => None,
            Self::Connection(error) => Some(error),
        }
    }
}
