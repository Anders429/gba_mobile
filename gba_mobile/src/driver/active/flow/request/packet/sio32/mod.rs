mod receive;
mod receive_error;
mod send;
mod wait_for_receive;

pub(super) use send::Send;

use super::{Payload, Sio};
use crate::mmio::serial::TransferLength;
use receive::Receive;
use receive_error::ReceiveError;
use wait_for_receive::WaitForReceive;

#[derive(Debug)]
pub(super) struct Sio32;

impl Sio for Sio32 {
    const TRANSFER_LENGTH: TransferLength = TransferLength::_32Bit;

    type Send<Payload>
        = Send<Payload>
    where
        Payload: self::Payload;
    type WaitForReceive<Payload>
        = WaitForReceive<Payload>
    where
        Payload: self::Payload;
    type Receive<Payload>
        = Receive<Payload>
    where
        Payload: self::Payload;
    type ReceiveError<Payload>
        = ReceiveError<Payload>
    where
        Payload: self::Payload;
}
