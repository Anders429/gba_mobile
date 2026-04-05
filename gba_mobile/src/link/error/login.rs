use super::connect;
use crate::{arrayvec, dns, driver, socket};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub struct Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    kind: Kind<Socket1, Socket2, Dns>,
}

impl<Socket1, Socket2, Dns> Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub(crate) fn id(error: arrayvec::error::Capacity<32>) -> Self {
        Self {
            kind: Kind::Id(error),
        }
    }

    pub(crate) fn password(error: arrayvec::error::Capacity<32>) -> Self {
        Self {
            kind: Kind::Password(error),
        }
    }
}

impl<Socket1, Socket2, Dns> Debug for Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.kind, formatter)
    }
}

impl<Socket1, Socket2, Dns> Display for Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.kind, formatter)
    }
}

impl<Socket1, Socket2, Dns> core::error::Error for Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<Socket1, Socket2, Dns> From<driver::error::link::Error<Socket1, Socket2, Dns>>
    for Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn from(error: driver::error::link::Error<Socket1, Socket2, Dns>) -> Self {
        Self {
            kind: Kind::Connect(error.into()),
        }
    }
}

impl<Socket1, Socket2, Dns> From<arrayvec::error::Capacity<32>> for Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn from(error: arrayvec::error::Capacity<32>) -> Self {
        Self {
            kind: Kind::Connect(error.into()),
        }
    }
}

enum Kind<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    Connect(connect::Error<Socket1, Socket2, Dns>),
    Id(arrayvec::error::Capacity<32>),
    Password(arrayvec::error::Capacity<32>),
}

impl<Socket1, Socket2, Dns> Debug for Kind<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Connect(error) => formatter.debug_tuple("Connect").field(error).finish(),
            Self::Id(error) => formatter.debug_tuple("Id").field(error).finish(),
            Self::Password(error) => formatter.debug_tuple("Password").field(error).finish(),
        }
    }
}

impl<Socket1, Socket2, Dns> Display for Kind<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Connect(_) => formatter.write_str("connection error"),
            Self::Id(_) => formatter.write_str("ID too long"),
            Self::Password(_) => formatter.write_str("password too long"),
        }
    }
}

impl<Socket1, Socket2, Dns> core::error::Error for Kind<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Connect(error) => Some(error),
            Self::Id(error) => Some(error),
            Self::Password(error) => Some(error),
        }
    }
}
