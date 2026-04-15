pub mod dns;

use crate::{config, driver, socket};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub struct Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    internal: driver::error::connection::Error<Socket1, Socket2, Dns, Config>,
}

impl<Socket1, Socket2, Dns, Config> Debug for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config> Display for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config> core::error::Error for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
    Config: config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<Socket1, Socket2, Dns, Config>
    From<driver::error::connection::Error<Socket1, Socket2, Dns, Config>>
    for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn from(error: driver::error::connection::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self { internal: error }
    }
}

impl<Socket1, Socket2, Dns, Config> From<driver::error::link::Error<Socket1, Socket2, Dns, Config>>
    for Error<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: crate::dns::Mode,
    Config: config::Mode,
{
    fn from(error: driver::error::link::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            internal: error.into(),
        }
    }
}
