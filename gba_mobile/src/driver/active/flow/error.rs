use super::{accept, connect, end, idle, login, reset, start, status, write_config};
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
    Login(login::Error),
    WriteConfig(write_config::Error),
    Status(status::Error),
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
            Self::Login(_) => formatter.write_str("error during login"),
            Self::WriteConfig(_) => formatter.write_str("error during write config"),
            Self::Status(_) => formatter.write_str("error during status"),
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
            Self::Login(error) => Some(error),
            Self::WriteConfig(error) => Some(error),
            Self::Status(error) => Some(error),
            Self::Idle(error) => Some(error),
        }
    }
}
