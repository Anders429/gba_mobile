use super::{accept, connect, end, idle, reset, start, status, write_config};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    Start(start::Timeout),
    End(end::Timeout),
    Reset(reset::Timeout),
    Accept(accept::Timeout),
    Connect(connect::Timeout),
    WriteConfig(write_config::Timeout),
    Status(status::Timeout),
    Idle(idle::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start(_) => formatter.write_str("timeout during start"),
            Self::End(_) => formatter.write_str("timeout during end"),
            Self::Reset(_) => formatter.write_str("timeout during reset"),
            Self::Accept(_) => formatter.write_str("timeout during accept"),
            Self::Connect(_) => formatter.write_str("timeout during connect"),
            Self::WriteConfig(_) => formatter.write_str("timeout during write config"),
            Self::Status(_) => formatter.write_str("timeout during status"),
            Self::Idle(_) => formatter.write_str("timeout during idle"),
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Start(timeout) => Some(timeout),
            Self::End(timeout) => Some(timeout),
            Self::Reset(timeout) => Some(timeout),
            Self::Accept(timeout) => Some(timeout),
            Self::Connect(timeout) => Some(timeout),
            Self::WriteConfig(timeout) => Some(timeout),
            Self::Status(timeout) => Some(timeout),
            Self::Idle(timeout) => Some(timeout),
        }
    }
}
