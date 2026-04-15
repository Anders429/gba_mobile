use super::{connection, link};
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

    pub(in crate::driver) fn canceled() -> Self {
        Self {
            kind: Kind::Canceled,
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

impl<Socket1, Socket2, Dns, Config> From<connection::Error<Socket1, Socket2, Dns, Config>>
    for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(error: connection::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            kind: Kind::Connection(error),
        }
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
            kind: Kind::Connection(error.into()),
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
            kind: Kind::Connection(error.into()),
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
    Superseded,
    NotFound,
    Canceled,
    Connection(connection::Error<Socket1, Socket2, Dns, Config>),
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
            Self::Superseded => formatter.write_str("Superseded"),
            Self::NotFound => formatter.write_str("NotFound"),
            Self::Canceled => formatter.write_str("Canceled"),
            Self::Connection(error) => formatter.debug_tuple("Connection").field(error).finish(),
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
            Self::Superseded => formatter.write_str("the DNS request was superseded"),
            Self::NotFound => formatter.write_str("domain lookup failed"),
            Self::Canceled => formatter.write_str("the DNS request was canceled"),
            Self::Connection(_) => formatter.write_str("connection error"),
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
            Self::Superseded => None,
            Self::NotFound => None,
            Self::Canceled => None,
            Self::Connection(error) => Some(error),
        }
    }
}
