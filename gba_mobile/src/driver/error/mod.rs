pub(crate) mod close_link;
pub(crate) mod connection;
pub(crate) mod link;

use super::active;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

/// All internal error states the driver can enter.
#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Timeout(active::Timeout),
    Error(active::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Timeout(_) => formatter.write_str("communication timed out"),
            Self::Error(_) => formatter.write_str("communication failed"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Timeout(timeout) => Some(timeout),
            Self::Error(error) => Some(error),
        }
    }
}
