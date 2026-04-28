mod receive;
mod receive_error;
mod send;
mod wait_for_receive;

pub(super) use send::Send;

use super::Sio;
use crate::mmio::serial::TransferLength;
use receive::Receive;
use receive_error::ReceiveError;
use wait_for_receive::WaitForReceive;

#[derive(Debug)]
pub(in crate::driver::active) struct Sio32;

impl Sio for Sio32 {
    const TRANSFER_LENGTH: TransferLength = TransferLength::_32Bit;

    type Send = Send;
    type WaitForReceive = WaitForReceive;
    type Receive = Receive;
    type ReceiveError = ReceiveError;
}
