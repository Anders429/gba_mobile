use crate::driver::active::flow::request::{idle, packet};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Timeout {
    TransferData(packet::Timeout),
    WriteToBuffer(idle::Timeout),
}

impl Display for Timeout {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::TransferData(_) => formatter.write_str("timeout while transferring data"),
            Self::WriteToBuffer(_) => formatter.write_str("timeout while writing to buffer"),
        }
    }
}

impl core::error::Error for Timeout {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::TransferData(timeout) => Some(timeout),
            Self::WriteToBuffer(timeout) => Some(timeout),
        }
    }
}
