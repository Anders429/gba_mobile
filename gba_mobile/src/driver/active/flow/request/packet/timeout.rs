use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    Serial,
    Packet,
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Serial => formatter.write_str("serial communication timed out"),
            Self::Packet => formatter.write_str("timeout while waiting for response packet"),
        }
    }
}

impl core::error::Error for Timeout {}
