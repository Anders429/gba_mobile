use crate::driver::active::flow::request::{packet, wait_for_idle};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    Reset(packet::Timeout),
    WaitForSio8(wait_for_idle::Timeout),
    EnableSio32(packet::Timeout),
    WaitForSio32(wait_for_idle::Timeout),
    ReadConfig1(packet::Timeout),
    ReadConfig2(packet::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Reset(_) => formatter.write_str("timeout while resetting session"),
            Self::WaitForSio8(_) => formatter.write_str(
                "timeout while waiting for adapter to enter idle state after resetting session",
            ),
            Self::EnableSio32(_) => formatter.write_str("timeout while enabling SIO32 mode"),
            Self::WaitForSio32(_) => formatter.write_str(
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
            Self::Reset(timeout) => Some(timeout),
            Self::WaitForSio8(timeout) => Some(timeout),
            Self::EnableSio32(timeout) => Some(timeout),
            Self::WaitForSio32(timeout) => Some(timeout),
            Self::ReadConfig1(timeout) => Some(timeout),
            Self::ReadConfig2(timeout) => Some(timeout),
        }
    }
}
