use super::{request, sink};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub(crate) enum Error {
    Request(request::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Request(_) => formatter.write_str("communication error"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
        }
    }
}
