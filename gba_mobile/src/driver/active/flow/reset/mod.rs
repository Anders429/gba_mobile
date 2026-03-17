mod error;
mod timeout;

use core::ptr;

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
    ReadConfig1(Packet<payload::ReadConfig>),
    ReadConfig2(Packet<payload::ReadConfig>),
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
            Self::ReadConfig1(packet) => packet
                .vblank()
                .map(Self::ReadConfig1)
                .map_err(Timeout::ReadConfig1),
            Self::ReadConfig2(packet) => packet
                .vblank()
                .map(Self::ReadConfig2)
                .map_err(Timeout::ReadConfig2),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::Reset(packet) => packet.timer(),
            Self::WaitForSio8(_) => {}
            Self::EnableSio32(packet) => packet.timer(),
            Self::WaitForSio32(_) => {}
            Self::ReadConfig1(packet) => packet.timer(),
            Self::ReadConfig2(packet) => packet.timer(),
        }
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
        phase: &mut Phase,
        config: &mut [u8; 256],
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
                        *transfer_length = response.payload.transfer_length;
                        unsafe {
                            SIOCNT.write_volatile(
                                SIOCNT.read_volatile().transfer_length(*transfer_length),
                            );
                        }
                        Some(Self::WaitForSio32(WaitForIdle::new(*transfer_length)))
                    }
                })
                .map_err(Error::EnableSio32),
            Self::WaitForSio32(wait_for_idle) => Ok(Some(wait_for_idle.serial().map_or_else(
                || {
                    Self::ReadConfig1(Packet::new(
                        payload::ReadConfig::FirstHalf,
                        *transfer_length,
                        timer,
                    ))
                },
                Self::WaitForSio32,
            ))),
            Self::ReadConfig1(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::ReadConfig1(packet)),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        unsafe {
                            ptr::copy_nonoverlapping(
                                response.payload.data().as_ptr(),
                                config.as_mut_ptr(),
                                128,
                            );
                        }
                        Some(Self::ReadConfig2(Packet::new(
                            payload::ReadConfig::SecondHalf,
                            *transfer_length,
                            timer,
                        )))
                    }
                })
                .map_err(Error::ReadConfig1),
            Self::ReadConfig2(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::ReadConfig2(packet)),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        unsafe {
                            ptr::copy_nonoverlapping(
                                response.payload.data().as_ptr(),
                                config.as_mut_ptr().add(128),
                                128,
                            );
                        }
                        *phase = Phase::Linked {
                            frame: 0,
                            connection_failure: None,
                        };
                        None
                    }
                })
                .map_err(Error::ReadConfig2),
        }
    }
}
