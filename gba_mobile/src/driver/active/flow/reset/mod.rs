mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::Phase,
    request::{Packet, WaitForIdle, packet::payload},
};
use crate::{
    Timer,
    driver::Adapter,
    mmio::serial::{SIOCNT, TransferLength},
};
use either::Either;

#[derive(Debug)]
pub(in super::super) enum Reset {
    Reset(Packet<payload::Reset>),
    WaitForSio8(WaitForIdle),
    EnableSio32(Packet<payload::EnableSio32>),
    WaitForSio32(WaitForIdle),
}

impl Reset {
    pub(super) fn new(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::Reset(Packet::new(payload::Reset, transfer_length, timer))
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::Reset(packet) => packet.vblank().map(Self::Reset).map_err(Timeout::Reset),
            Self::WaitForSio8(wait_for_idle) => wait_for_idle
                .vblank()
                .map(Self::WaitForSio8)
                .map_err(Timeout::WaitForSio8),
            Self::EnableSio32(packet) => packet
                .vblank()
                .map(Self::EnableSio32)
                .map_err(Timeout::EnableSio32),
            Self::WaitForSio32(wait_for_idle) => wait_for_idle
                .vblank()
                .map(Self::WaitForSio32)
                .map_err(Timeout::WaitForSio32),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::Reset(packet) => packet.timer(),
            Self::WaitForSio8(_) => {}
            Self::EnableSio32(packet) => packet.timer(),
            Self::WaitForSio32(_) => {}
        }
    }

    pub(super) fn serial(
        self,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
        timer: Timer,
        phase: &mut Phase,
    ) -> Result<Option<Self>, Error> {
        match self {
            Self::Reset(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::Reset(packet)),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        *transfer_length = TransferLength::_8Bit;
                        unsafe {
                            SIOCNT.write_volatile(
                                SIOCNT.read_volatile().transfer_length(*transfer_length),
                            );
                        }
                        Some(Self::WaitForSio8(WaitForIdle::new(*transfer_length)))
                    }
                })
                .map_err(Error::Reset),
            Self::WaitForSio8(wait_for_idle) => Ok(Some(wait_for_idle.serial().map_or_else(
                || Self::EnableSio32(Packet::new(payload::EnableSio32, *transfer_length, timer)),
                Self::WaitForSio8,
            ))),
            Self::EnableSio32(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::EnableSio32(packet)),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        match response.payload {
                            payload::enable_sio32::ReceiveParsed::EnableSio32 => {
                                *transfer_length = TransferLength::_32Bit
                            }
                            payload::enable_sio32::ReceiveParsed::DisableSio32 => {
                                *transfer_length = TransferLength::_8Bit
                            }
                        };
                        unsafe {
                            SIOCNT.write_volatile(
                                SIOCNT.read_volatile().transfer_length(*transfer_length),
                            );
                        }
                        Some(Self::WaitForSio32(WaitForIdle::new(*transfer_length)))
                    }
                })
                .map_err(Error::EnableSio32),
            Self::WaitForSio32(wait_for_idle) => Ok(wait_for_idle.serial().map_or_else(
                || {
                    *phase = Phase::Linked {
                        frame: 0,
                        connection_failure: None,
                    };
                    None
                },
                |wait_for_idle| Some(Self::WaitForSio32(wait_for_idle)),
            )),
        }
    }
}
