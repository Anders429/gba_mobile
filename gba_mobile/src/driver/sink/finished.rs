use crate::{driver::command, mmio::serial::TransferLength};

#[derive(Debug)]
pub(in crate::driver) enum Finished {
    Success,
    TransferLength(TransferLength),
    CommandError(command::Error),
}
