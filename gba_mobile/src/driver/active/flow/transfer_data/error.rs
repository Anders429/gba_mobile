use crate::driver::active::flow::request::{idle, packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error {
    TransferData(packet::Error<payload::TransferData>),
    Idle(idle::Error),
    // TODO: Add the error here.
    WriteToBuffer,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::TransferData(_) => formatter.write_str("error while transferring data"),
            Self::Idle(_) => formatter.write_str("error while idling"),
            Self::WriteToBuffer => formatter.write_str("error while writing to buffer"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::TransferData(error) => Some(error),
            Self::Idle(error) => Some(error),
            Self::WriteToBuffer => None,
        }
    }
}
