mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::request::{Packet, packet::payload};
use crate::{
    Timer,
    driver::{Adapter, frames},
    mmio::serial::{SIOCNT, TransferLength},
};
use either::Either;

#[derive(Debug)]
pub(in super::super) enum End {
    EndSession(Packet<payload::EndSession>),
    WaitForSio8(u8),
}

impl End {
    pub(super) fn new(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::EndSession(Packet::new(payload::EndSession, transfer_length, timer))
    }

    pub(super) fn vblank(&mut self) -> Result<bool, Timeout> {
        match self {
            Self::EndSession(packet) => packet.vblank().map(|_| true).map_err(Timeout::EndSession),
            Self::WaitForSio8(frame) => {
                if *frame >= frames::ONE_HUNDRED_MILLISECONDS {
                    // We have waited sufficiently long enough to fully close the active state..
                    Ok(false)
                } else {
                    *frame = frame.saturating_add(1);
                    Ok(true)
                }
            }
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::EndSession(packet) => packet.timer(),
            Self::WaitForSio8(_) => {}
        }
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
    ) -> Result<Self, Error> {
        match self {
            Self::EndSession(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Self::EndSession(packet),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        *transfer_length = TransferLength::_8Bit;
                        unsafe {
                            SIOCNT.write_volatile(
                                SIOCNT.read_volatile().transfer_length(*transfer_length),
                            );
                        }
                        Self::WaitForSio8(0)
                    }
                })
                .map_err(Error::EndSession),
            Self::WaitForSio8(frame) => Ok(Self::WaitForSio8(frame)),
        }
    }
}
