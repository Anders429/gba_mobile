use crate::driver::active::flow::request::{packet, wait_for_idle};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    Wake(wait_for_idle::Timeout),
    BeginSession(packet::Timeout),
    Sio32(packet::Timeout),
    WaitForIdle(wait_for_idle::Timeout),
    ReadConfig1(packet::Timeout),
    ReadConfig2(packet::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Wake(_) => formatter.write_str("timeout while waking adapter"),
            Self::BeginSession(_) => formatter.write_str("timeout while beginning session"),
            Self::Sio32(_) => formatter.write_str("timeout while enabling SIO32 mode"),
            Self::WaitForIdle(_) => formatter.write_str(
                "timeout while waiting for adapter to enter idle state after enabling SIO32 mode",
            ),
            Self::ReadConfig1(_) => {
                formatter.write_str("timeout while reading first half of config")
            }
            Self::ReadConfig2(_) => {
                formatter.write_str("timeout while reading second half of config")
            }
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Wake(timeout) => Some(timeout),
            Self::BeginSession(timeout) => Some(timeout),
            Self::Sio32(timeout) => Some(timeout),
            Self::WaitForIdle(timeout) => Some(timeout),
            Self::ReadConfig1(timeout) => Some(timeout),
            Self::ReadConfig2(timeout) => Some(timeout),
        }
    }
}
