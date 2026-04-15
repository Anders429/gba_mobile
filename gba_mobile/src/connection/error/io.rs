use crate::driver;
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub struct P2p<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    internal: driver::error::connection_io::Error<IoError, Socket1, Socket2, Dns, Config>,
}

impl<IoError, Socket1, Socket2, Dns, Config> Debug for P2p<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: Debug,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.internal, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> Display for P2p<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: Display,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> core::error::Error
    for P2p<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: core::error::Error + 'static,
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
    Config: crate::config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<IoError, Socket1, Socket2, Dns, Config>
    From<driver::error::connection_io::Error<IoError, Socket1, Socket2, Dns, Config>>
    for P2p<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(
        error: driver::error::connection_io::Error<IoError, Socket1, Socket2, Dns, Config>,
    ) -> Self {
        Self { internal: error }
    }
}

impl<IoError, Socket1, Socket2, Dns, Config>
    From<driver::error::link::Error<Socket1, Socket2, Dns, Config>>
    for P2p<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(error: driver::error::link::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            internal: error.into(),
        }
    }
}

pub struct Socket<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    internal: driver::error::socket_io::Error<IoError, Socket1, Socket2, Dns, Config>,
}

impl<IoError, Socket1, Socket2, Dns, Config> Debug
    for Socket<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: Debug,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.internal, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> Display
    for Socket<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: Display,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.internal, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> core::error::Error
    for Socket<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: core::error::Error + 'static,
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
    Config: crate::config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.internal.source()
    }
}

impl<IoError, Socket1, Socket2, Dns, Config>
    From<driver::error::socket_io::Error<IoError, Socket1, Socket2, Dns, Config>>
    for Socket<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(
        error: driver::error::socket_io::Error<IoError, Socket1, Socket2, Dns, Config>,
    ) -> Self {
        Self { internal: error }
    }
}

impl<IoError, Socket1, Socket2, Dns, Config>
    From<driver::error::link::Error<Socket1, Socket2, Dns, Config>>
    for Socket<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(error: driver::error::link::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            internal: error.into(),
        }
    }
}
