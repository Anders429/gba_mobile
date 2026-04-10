pub mod io;

use crate::{dns, driver, socket};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub struct P2p<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    internal: driver::error::connection::Error<Socket1, Socket2, Dns>,
}

impl<Socket1, Socket2, Dns> Debug for P2p<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns> Display for P2p<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns> core::error::Error for P2p<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<Socket1, Socket2, Dns> From<driver::error::connection::Error<Socket1, Socket2, Dns>>
    for P2p<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn from(error: driver::error::connection::Error<Socket1, Socket2, Dns>) -> Self {
        Self { internal: error }
    }
}

impl<Socket1, Socket2, Dns> From<driver::error::link::Error<Socket1, Socket2, Dns>>
    for P2p<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
{
    fn from(error: driver::error::link::Error<Socket1, Socket2, Dns>) -> Self {
        Self {
            internal: error.into(),
        }
    }
}

pub struct Socket<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    internal: driver::error::socket::Error<Socket1, Socket2, Dns>,
}

impl<Socket1, Socket2, Dns> Debug for Socket<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns> Display for Socket<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns> core::error::Error for Socket<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<Socket1, Socket2, Dns> From<driver::error::socket::Error<Socket1, Socket2, Dns>>
    for Socket<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn from(error: driver::error::socket::Error<Socket1, Socket2, Dns>) -> Self {
        Self { internal: error }
    }
}

impl<Socket1, Socket2, Dns> From<driver::error::link::Error<Socket1, Socket2, Dns>>
    for Socket<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
{
    fn from(error: driver::error::link::Error<Socket1, Socket2, Dns>) -> Self {
        Self {
            internal: error.into(),
        }
    }
}
