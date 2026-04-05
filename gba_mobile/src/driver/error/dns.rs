use super::{connection, link};
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
    Superseded,
    NotFound,
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
            Self::Superseded => formatter.write_str("Superseded"),
            Self::NotFound => formatter.write_str("NotFound"),
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
            Self::Superseded => formatter.write_str("the DNS request was superseded"),
            Self::NotFound => formatter.write_str("domain lookup failed"),
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
            Self::Superseded => None,
            Self::NotFound => None,
            Self::Connection(error) => Some(error),
        }
    }
}
