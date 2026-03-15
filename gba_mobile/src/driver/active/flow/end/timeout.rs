use crate::driver::active::flow::request::{packet, wait_for_idle};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    EndSession(packet::Timeout),
    WaitForIdle(wait_for_idle::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::EndSession(_) => formatter.write_str("timeout while ending session"),
            Self::WaitForIdle(_) => formatter.write_str(
                "timeout while waiting for adapter to enter idle state after ending session",
            ),
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::EndSession(timeout) => Some(timeout),
            Self::WaitForIdle(timeout) => Some(timeout),
        }
    }
}
