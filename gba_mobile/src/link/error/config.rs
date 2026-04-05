use crate::{dns, driver, link, socket};
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

pub struct Error<ConfigError, Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    kind: Kind<ConfigError, Socket1, Socket2, Dns>,
}

impl<ConfigError, Socket1, Socket2, Dns> Error<ConfigError, Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    pub(crate) fn config_error(config_error: ConfigError) -> Self {
        Self {
            kind: Kind::Config(config_error),
        }
    }
}

impl<ConfigError, Socket1, Socket2, Dns> Debug for Error<ConfigError, Socket1, Socket2, Dns>
where
    ConfigError: Debug,
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.kind, formatter)
    }
}

impl<ConfigError, Socket1, Socket2, Dns> Display for Error<ConfigError, Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.kind, formatter)
    }
}

impl<ConfigError, Socket1, Socket2, Dns> core::error::Error
    for Error<ConfigError, Socket1, Socket2, Dns>
where
    ConfigError: core::error::Error + 'static,
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<ConfigError, Socket1, Socket2, Dns> From<driver::error::link::Error<Socket1, Socket2, Dns>>
    for Error<ConfigError, Socket1, Socket2, Dns>
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

enum Kind<ConfigError, Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    Link(link::Error<Socket1, Socket2, Dns>),
    Config(ConfigError),
}

impl<ConfigError, Socket1, Socket2, Dns> Debug for Kind<ConfigError, Socket1, Socket2, Dns>
where
    ConfigError: Debug,
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Link(error) => formatter.debug_tuple("Link").field(error).finish(),
            Self::Config(error) => formatter.debug_tuple("Config").field(error).finish(),
        }
    }
}

impl<ConfigError, Socket1, Socket2, Dns> Display for Kind<ConfigError, Socket1, Socket2, Dns>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Link(_) => formatter.write_str("Mobile Adapter link connection error"),
            Self::Config(_) => formatter.write_str("config parse error"),
        }
    }
}

impl<ConfigError, Socket1, Socket2, Dns> core::error::Error
    for Kind<ConfigError, Socket1, Socket2, Dns>
where
    ConfigError: core::error::Error + 'static,
    Socket1: socket::Slot + 'static,
    Socket2: socket::Slot + 'static,
    Dns: dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Link(error) => Some(error),
            Self::Config(error) => Some(error),
        }
    }
}
