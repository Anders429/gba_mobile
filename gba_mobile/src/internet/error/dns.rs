use crate::{arrayvec, config, driver, internet, socket};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub struct Error<Socket1, Socket2, Dns, Config, const MAX_LEN: usize>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    kind: Kind<Socket1, Socket2, Dns, Config, MAX_LEN>,
}

impl<Socket1, Socket2, Dns, Config, const MAX_LEN: usize> Debug
    for Error<Socket1, Socket2, Dns, Config, MAX_LEN>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.kind, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config, const MAX_LEN: usize> Display
    for Error<Socket1, Socket2, Dns, Config, MAX_LEN>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.kind, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config, const MAX_LEN: usize> core::error::Error
    for Error<Socket1, Socket2, Dns, Config, MAX_LEN>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
    Config: config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<Socket1, Socket2, Dns, Config, const MAX_LEN: usize>
    From<driver::error::connection::Error<Socket1, Socket2, Dns, Config>>
    for Error<Socket1, Socket2, Dns, Config, MAX_LEN>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn from(error: driver::error::connection::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

impl<Socket1, Socket2, Dns, Config, const MAX_LEN: usize>
    From<driver::error::link::Error<Socket1, Socket2, Dns, Config>>
    for Error<Socket1, Socket2, Dns, Config, MAX_LEN>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn from(error: driver::error::link::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

impl<Socket1, Socket2, Dns, Config, const MAX_LEN: usize> From<arrayvec::error::Capacity<MAX_LEN>>
    for Error<Socket1, Socket2, Dns, Config, MAX_LEN>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn from(error: arrayvec::error::Capacity<MAX_LEN>) -> Self {
        Self {
            kind: Kind::Capacity(error),
        }
    }
}

enum Kind<Socket1, Socket2, Dns, Config, const MAX_LEN: usize>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    Capacity(arrayvec::error::Capacity<MAX_LEN>),
    Connection(internet::Error<Socket1, Socket2, Dns, Config>),
}

impl<Socket1, Socket2, Dns, Config, const MAX_LEN: usize> Debug
    for Kind<Socket1, Socket2, Dns, Config, MAX_LEN>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Capacity(error) => formatter.debug_tuple("Capacity").field(error).finish(),
            Self::Connection(error) => formatter.debug_tuple("Connection").field(error).finish(),
        }
    }
}

impl<Socket1, Socket2, Dns, Config, const MAX_LEN: usize> Display
    for Kind<Socket1, Socket2, Dns, Config, MAX_LEN>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Capacity(_) => formatter.write_str("could not create domain name lookup request"),
            Self::Connection(_) => formatter.write_str("Mobile Adapter connection error"),
        }
    }
}

impl<Socket1, Socket2, Dns, Config, const MAX_LEN: usize> core::error::Error
    for Kind<Socket1, Socket2, Dns, Config, MAX_LEN>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
    Config: config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Capacity(error) => Some(error),
            Self::Connection(error) => Some(error),
        }
    }
}
