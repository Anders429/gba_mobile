use super::{accept, connect, end, idle, reset, start};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    Start(start::Error),
    End(end::Error),
    Reset(reset::Error),
    Accept(accept::Error),
    Connect(connect::Error),
    Idle(idle::Error),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start(_) => formatter.write_str("error during start"),
            Self::End(_) => formatter.write_str("error during end"),
            Self::Reset(_) => formatter.write_str("error during reset"),
            Self::Accept(_) => formatter.write_str("error during accept"),
            Self::Connect(_) => formatter.write_str("error during connect"),
            Self::Idle(_) => formatter.write_str("error during idle"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Start(error) => Some(error),
            Self::End(error) => Some(error),
            Self::Reset(error) => Some(error),
            Self::Accept(error) => Some(error),
            Self::Connect(error) => Some(error),
            Self::Idle(error) => Some(error),
        }
    }
}
