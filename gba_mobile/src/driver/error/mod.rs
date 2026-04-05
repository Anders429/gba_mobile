pub(crate) mod close_link;
pub(crate) mod connection;
pub(crate) mod connection_io;
pub(crate) mod dns;
pub(crate) mod link;
pub(crate) mod socket;
pub(crate) mod socket_io;

use super::active;
use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

/// All internal error states the driver can enter.
pub(in crate::driver) enum Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    Timeout(active::Timeout),
    Error(active::Error<Socket1, Socket2, Dns>),
}

impl<Socket1, Socket2, Dns> Clone for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn clone(&self) -> Self {
        match self {
            Self::Timeout(timeout) => Self::Timeout(timeout.clone()),
            Self::Error(error) => Self::Error(error.clone()),
        }
    }
}

impl<Socket1, Socket2, Dns> Debug for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Timeout(timeout) => formatter.debug_tuple("Timeout").field(timeout).finish(),
            Self::Error(error) => formatter.debug_tuple("Error").field(error).finish(),
        }
    }
}

impl<Socket1, Socket2, Dns> Display for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot,
    Socket2: crate::socket::Slot,
    Dns: crate::dns::Mode,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Timeout(_) => formatter.write_str("communication timed out"),
            Self::Error(_) => formatter.write_str("communication failed"),
        }
    }
}

impl<Socket1, Socket2, Dns> core::error::Error for Error<Socket1, Socket2, Dns>
where
    Socket1: crate::socket::Slot + 'static,
    Socket2: crate::socket::Slot + 'static,
    Dns: crate::dns::Mode + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Timeout(timeout) => Some(timeout),
            Self::Error(error) => Some(error),
        }
    }
}
