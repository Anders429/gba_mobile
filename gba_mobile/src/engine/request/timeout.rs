use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::engine) enum Timeout {
    Packet,
    WaitForIdle,
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Packet => formatter.write_str("timeout while waiting for packet response"),
            Self::WaitForIdle => formatter
                .write_str("timeout while waiting for the adapter to return an idle byte (0x4b)"),
        }
    }
}

impl core::error::Error for Timeout {}
