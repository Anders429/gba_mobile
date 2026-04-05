use crate::driver::active::flow::request::{idle, packet, packet::payload};
use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Debug)]
pub(in crate::driver) enum Error<BufferError> {
    TransferData(packet::Error<payload::TransferData>),
    Idle(idle::Error),
    WriteToBuffer(BufferError),
}

impl<BufferError> Display for Error<BufferError> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::TransferData(_) => formatter.write_str("error while transferring data"),
            Self::Idle(_) => formatter.write_str("error while idling"),
            Self::WriteToBuffer(_) => formatter.write_str("error while writing to buffer"),
        }
    }
}

impl<BufferError> core::error::Error for Error<BufferError>
where
    BufferError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::TransferData(error) => Some(error),
            Self::Idle(error) => Some(error),
            Self::WriteToBuffer(error) => Some(error),
        }
    }
}
