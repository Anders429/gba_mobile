use crate::driver;
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub struct P2p<IoError, Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    internal: driver::error::connection_io::Error<IoError, Socket1, Socket2, Dns>,
}

impl<IoError, Socket1, Socket2, Dns> Debug for P2p<IoError, Socket1, Socket2, Dns>
where
    IoError: Debug,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.internal, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns> Display for P2p<IoError, Socket1, Socket2, Dns>
where
    IoError: Display,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns> core::error::Error for P2p<IoError, Socket1, Socket2, Dns>
where
    IoError: core::error::Error + 'static,
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<IoError, Socket1, Socket2, Dns>
    From<driver::error::connection_io::Error<IoError, Socket1, Socket2, Dns>>
    for P2p<IoError, Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn from(error: driver::error::connection_io::Error<IoError, Socket1, Socket2, Dns>) -> Self {
        Self { internal: error }
    }
}

pub struct Socket<IoError, Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    internal: driver::error::socket_io::Error<IoError, Socket1, Socket2, Dns>,
}

impl<IoError, Socket1, Socket2, Dns> Debug for Socket<IoError, Socket1, Socket2, Dns>
where
    IoError: Debug,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.internal, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns> Display for Socket<IoError, Socket1, Socket2, Dns>
where
    IoError: Display,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns> core::error::Error for Socket<IoError, Socket1, Socket2, Dns>
where
    IoError: core::error::Error + 'static,
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<IoError, Socket1, Socket2, Dns>
    From<driver::error::socket_io::Error<IoError, Socket1, Socket2, Dns>>
    for Socket<IoError, Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn from(error: driver::error::socket_io::Error<IoError, Socket1, Socket2, Dns>) -> Self {
        Self { internal: error }
    }
}
