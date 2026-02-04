use crate::{engine::command, mmio::serial::TransferLength};

#[derive(Debug)]
pub(in crate::engine) enum Finished {
    Success,
    TransferLength(TransferLength),
    CommandError(command::Error),
}
