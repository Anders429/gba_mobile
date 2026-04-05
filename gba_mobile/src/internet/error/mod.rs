pub mod dns;

use crate::{driver, socket};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub struct Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
{
    internal: driver::error::connection::Error<Socket1, Socket2, Dns>,
}

impl<Socket1, Socket2, Dns> Debug for Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns> Display for Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns> core::error::Error for Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<Socket1, Socket2, Dns> From<driver::error::connection::Error<Socket1, Socket2, Dns>>
    for Error<Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
{
    fn from(error: driver::error::connection::Error<Socket1, Socket2, Dns>) -> Self {
        Self { internal: error }
    }
}
