use super::link;
use crate::driver::active::ConnectionFailure;
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub(crate) struct Error<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    kind: Kind<Socket1, Socket2, Dns, Config>,
}

impl<Socket1, Socket2, Dns, Config> Error<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    pub(in crate::driver) fn closed() -> Self {
        Self { kind: Kind::Closed }
    }

    pub(in crate::driver) fn superseded() -> Self {
        Self {
            kind: Kind::Superseded,
        }
    }
}

impl<Socket1, Socket2, Dns, Config> Debug for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.kind, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config> Display for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.kind, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config> core::error::Error for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
    Config: crate::config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<Socket1, Socket2, Dns, Config> From<ConnectionFailure> for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(error: ConnectionFailure) -> Self {
        Self {
            kind: Kind::Failure(error),
        }
    }
}

impl<Socket1, Socket2, Dns, Config> From<link::Error<Socket1, Socket2, Dns, Config>>
    for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(error: link::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            kind: Kind::Link(error),
        }
    }
}

enum Kind<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    Closed,
    Superseded,
    Failure(ConnectionFailure),
    Link(link::Error<Socket1, Socket2, Dns, Config>),
}

impl<Socket1, Socket2, Dns, Config> Debug for Kind<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("Closed"),
            Self::Superseded => formatter.write_str("Superseded"),
            Self::Failure(error) => formatter.debug_tuple("Failure").field(error).finish(),
            Self::Link(error) => formatter.debug_tuple("Link").field(error).finish(),
        }
    }
}

impl<Socket1, Socket2, Dns, Config> Display for Kind<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => formatter.write_str("the connection was closed"),
            Self::Superseded => formatter.write_str("the connection was superseded"),
            Self::Failure(_) => formatter.write_str("failed to establish connection"),
            Self::Link(_) => formatter.write_str("link error"),
        }
    }
}

impl<Socket1, Socket2, Dns, Config> core::error::Error for Kind<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
    Config: crate::config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Closed => None,
            Self::Superseded => None,
            Self::Failure(error) => Some(error),
            Self::Link(error) => Some(error),
        }
    }
}
