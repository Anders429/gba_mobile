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

impl<Socket1, Socket2, Dns, Config> From<super::Error<Socket1, Socket2, Dns, Config>>
    for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(error: super::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            kind: Kind::Driver(error),
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
    Driver(super::Error<Socket1, Socket2, Dns, Config>),
    Closed,
    Superseded,
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
            Self::Driver(error) => formatter.debug_tuple("Driver").field(error).finish(),
            Self::Closed => formatter.write_str("Closed"),
            Self::Superseded => formatter.write_str("Superseded"),
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
            Self::Driver(_) => formatter.write_str("the driver is in an error state"),
            Self::Closed => formatter.write_str("the link connection was closed"),
            Self::Superseded => formatter.write_str("the link connection was superseded"),
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
            Self::Driver(error) => Some(error),
            Self::Closed => None,
            Self::Superseded => None,
        }
    }
}
