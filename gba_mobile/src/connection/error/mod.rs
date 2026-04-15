pub mod io;

use crate::{config, dns, driver, socket};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub struct P2p<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    internal: driver::error::connection::Error<Socket1, Socket2, Dns, Config>,
}

impl<Socket1, Socket2, Dns, Config> Debug for P2p<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config> Display for P2p<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config> core::error::Error for P2p<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: dns::Mode + 'static,
    Config: config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<Socket1, Socket2, Dns, Config>
    From<driver::error::connection::Error<Socket1, Socket2, Dns, Config>>
    for P2p<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn from(error: driver::error::connection::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self { internal: error }
    }
}

impl<Socket1, Socket2, Dns, Config> From<driver::error::link::Error<Socket1, Socket2, Dns, Config>>
    for P2p<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn from(error: driver::error::link::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            internal: error.into(),
        }
    }
}

pub struct Socket<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    internal: driver::error::socket::Error<Socket1, Socket2, Dns, Config>,
}

impl<Socket1, Socket2, Dns, Config> Debug for Socket<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config> Display for Socket<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<Socket1, Socket2, Dns, Config> core::error::Error for Socket<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: dns::Mode + 'static,
    Config: config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<Socket1, Socket2, Dns, Config>
    From<driver::error::socket::Error<Socket1, Socket2, Dns, Config>>
    for Socket<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn from(error: driver::error::socket::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self { internal: error }
    }
}

impl<Socket1, Socket2, Dns, Config> From<driver::error::link::Error<Socket1, Socket2, Dns, Config>>
    for Socket<Socket1, Socket2, Dns, Config>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
{
    fn from(error: driver::error::link::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            internal: error.into(),
        }
    }
}
