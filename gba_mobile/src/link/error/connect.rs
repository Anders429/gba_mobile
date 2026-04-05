use crate::{arrayvec, dns, driver, link, socket};
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
            kind: Kind::Link(error.into()),
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
            kind: Kind::PhoneNumber(error),
        }
    }
}

enum Kind<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    Link(link::Error<Socket1, Socket2, Dns>),
    PhoneNumber(arrayvec::error::Capacity<32>),
}

impl<Socket1, Socket2, Dns> Debug for Kind<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Link(error) => formatter.debug_tuple("Link").field(error).finish(),
            Self::PhoneNumber(error) => formatter.debug_tuple("PhoneNumber").field(error).finish(),
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
            Self::Link(_) => formatter.write_str("Mobile Adapter link connection error"),
            Self::PhoneNumber(_) => formatter.write_str("phone number error"),
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
            Self::Link(error) => Some(error),
            Self::PhoneNumber(error) => Some(error),
        }
    }
}
