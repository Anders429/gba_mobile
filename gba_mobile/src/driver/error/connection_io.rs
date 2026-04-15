use super::{connection, link};
use crate::driver::active::ConnectionFailure;
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub(crate) struct Error<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    kind: Kind<IoError, Socket1, Socket2, Dns, Config>,
}

impl<IoError, Socket1, Socket2, Dns, Config> Error<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    pub(in crate::driver) fn io(error: IoError) -> Self {
        Self {
            kind: Kind::Io(error),
        }
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> Debug for Error<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: Debug,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.kind, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> Display
    for Error<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: Display,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.kind, formatter)
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> core::error::Error
    for Error<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: core::error::Error + 'static,
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
    Config: crate::config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> From<connection::Error<Socket1, Socket2, Dns, Config>>
    for Error<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(error: connection::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            kind: Kind::Connection(error),
        }
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> From<ConnectionFailure>
    for Error<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(error: ConnectionFailure) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> From<link::Error<Socket1, Socket2, Dns, Config>>
    for Error<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn from(error: link::Error<Socket1, Socket2, Dns, Config>) -> Self {
        Self {
            kind: Kind::Connection(error.into()),
        }
    }
}

enum Kind<IoError, Socket1, Socket2, Dns, Config>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    Io(IoError),
    Connection(connection::Error<Socket1, Socket2, Dns, Config>),
}

impl<IoError, Socket1, Socket2, Dns, Config> Debug for Kind<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: Debug,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Io(error) => formatter.debug_tuple("Io").field(error).finish(),
            Self::Connection(error) => formatter.debug_tuple("Connection").field(error).finish(),
        }
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> Display
    for Kind<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: Display,
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
    Config: crate::config::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Io(_) => formatter.write_str("io error"),
            Self::Connection(_) => formatter.write_str("connection error"),
        }
    }
}

impl<IoError, Socket1, Socket2, Dns, Config> core::error::Error
    for Kind<IoError, Socket1, Socket2, Dns, Config>
where
    IoError: core::error::Error + 'static,
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
    Config: crate::config::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Connection(error) => Some(error),
        }
    }
}
