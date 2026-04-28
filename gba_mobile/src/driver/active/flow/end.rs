use super::request::{Packet, packet, packet::payload};
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
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
    ) -> Self {
        Self::EndSession(Packet::new(
            payload::EndSession::new(packet_data),
            transfer_length,
            timer,
        ))
    }

    pub(super) fn vblank(&mut self) -> Result<bool, packet::Timeout> {
        match self {
            Self::EndSession(packet) => packet.vblank().map(|_| true),
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

    pub(super) fn timer(&mut self, packet_data: &packet::Data) {
        match self {
            Self::EndSession(packet) => packet.timer(packet_data),
            Self::WaitForSio8(_) => {}
        }
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        packet_data: &mut packet::Data,
        transfer_length: &mut TransferLength,
    ) -> Result<Self, packet::Error<payload::EndSession>> {
        match self {
            Self::EndSession(packet) => {
                packet
                    .serial(timer, packet_data)
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
            }
            Self::WaitForSio8(frame) => Ok(Self::WaitForSio8(frame)),
        }
    }
}
