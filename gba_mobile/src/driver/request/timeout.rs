use super::packet;
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    Packet(packet::Timeout),
    WaitForIdle,
    Idle,
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Packet(_) => formatter.write_str("timeout in packet communication"),
            Self::WaitForIdle => formatter
                .write_str("timeout while waiting for the adapter to return an idle byte (0xd2)"),
            Self::Idle => formatter.write_str("timeout while waiting for idle response (0xd2)"),
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Packet(timeout) => Some(timeout),
            Self::WaitForIdle => None,
            Self::Idle => None,
        }
    }
}
