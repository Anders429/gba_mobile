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
    Generation, Timer,
    driver::Adapter,
    mmio::serial::{SIOCNT, TransferLength},
};
use either::Either;

#[derive(Debug)]
enum State {
    Reset(Packet<payload::Reset>),
    WaitForSio8(WaitForIdle),
    EnableSio32(Packet<payload::EnableSio32>),
    WaitForSio32(WaitForIdle),
    ReadConfig1(Packet<payload::ReadConfig>),
    ReadConfig2(Packet<payload::ReadConfig>),
}

#[derive(Debug)]
pub(in super::super) struct Reset {
    state: State,
    link_generation: Generation,
}

impl Reset {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        link_generation: Generation,
    ) -> Self {
        Self {
            state: State::Reset(Packet::new(payload::Reset, transfer_length, timer)),
            link_generation,
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self.state {
            State::Reset(packet) => packet
                .vblank()
                .map(|packet| Self {
                    state: State::Reset(packet),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::Reset),
            State::WaitForSio8(wait_for_idle) => wait_for_idle
                .vblank()
                .map(|wait_for_idle| Self {
                    state: State::WaitForSio8(wait_for_idle),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::WaitForSio8),
            State::EnableSio32(packet) => packet
                .vblank()
                .map(|packet| Self {
                    state: State::EnableSio32(packet),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::EnableSio32),
            State::WaitForSio32(wait_for_idle) => wait_for_idle
                .vblank()
                .map(|wait_for_idle| Self {
                    state: State::WaitForSio32(wait_for_idle),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::WaitForSio32),
            State::ReadConfig1(packet) => packet
                .vblank()
                .map(|packet| Self {
                    state: State::ReadConfig1(packet),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::ReadConfig1),
            State::ReadConfig2(packet) => packet
                .vblank()
                .map(|packet| Self {
                    state: State::ReadConfig2(packet),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::ReadConfig2),
        }
    }

    pub(super) fn timer(&mut self) {
        match &mut self.state {
            State::Reset(packet) => packet.timer(),
            State::WaitForSio8(_) => {}
            State::EnableSio32(packet) => packet.timer(),
            State::WaitForSio32(_) => {}
            State::ReadConfig1(packet) => packet.timer(),
            State::ReadConfig2(packet) => packet.timer(),
        }
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
        phase: &mut Phase,
        config: &mut [u8; 256],
        link_generation: Generation,
    ) -> Result<Option<Self>, Error> {
        match self.state {
            State::Reset(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self {
                        state: State::Reset(packet),
                        link_generation: self.link_generation,
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        *transfer_length = TransferLength::_8Bit;
                        unsafe {
                            SIOCNT.write_volatile(
                                SIOCNT.read_volatile().transfer_length(*transfer_length),
                            );
                        }
                        Some(Self {
                            state: State::WaitForSio8(WaitForIdle::new(*transfer_length)),
                            link_generation: self.link_generation,
                        })
                    }
                })
                .map_err(Error::Reset),
            State::WaitForSio8(wait_for_idle) => Ok(Some(wait_for_idle.serial().map_or_else(
                || Self {
                    state: State::EnableSio32(Packet::new(
                        payload::EnableSio32,
                        *transfer_length,
                        timer,
                    )),
                    link_generation: self.link_generation,
                },
                |wait_for_idle| Self {
                    state: State::WaitForSio8(wait_for_idle),
                    link_generation: self.link_generation,
                },
            ))),
            State::EnableSio32(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self {
                        state: State::EnableSio32(packet),
                        link_generation: self.link_generation,
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        *transfer_length = response.payload.transfer_length;
                        unsafe {
                            SIOCNT.write_volatile(
                                SIOCNT.read_volatile().transfer_length(*transfer_length),
                            );
                        }
                        Some(Self {
                            state: State::WaitForSio32(WaitForIdle::new(*transfer_length)),
                            link_generation: self.link_generation,
                        })
                    }
                })
                .map_err(Error::EnableSio32),
            State::WaitForSio32(wait_for_idle) => Ok(Some(wait_for_idle.serial().map_or_else(
                || Self {
                    state: State::ReadConfig1(Packet::new(
                        payload::ReadConfig::FirstHalf,
                        *transfer_length,
                        timer,
                    )),
                    link_generation: self.link_generation,
                },
                |wait_for_idle| Self {
                    state: State::WaitForSio32(wait_for_idle),
                    link_generation: self.link_generation,
                },
            ))),
            State::ReadConfig1(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self {
                        state: State::ReadConfig1(packet),
                        link_generation: self.link_generation,
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        unsafe {
                            ptr::copy_nonoverlapping(
                                response.payload.data().as_ptr(),
                                config.as_mut_ptr(),
                                128,
                            );
                        }
                        Some(Self {
                            state: State::ReadConfig2(Packet::new(
                                payload::ReadConfig::SecondHalf,
                                *transfer_length,
                                timer,
                            )),
                            link_generation: self.link_generation,
                        })
                    }
                })
                .map_err(Error::ReadConfig1),
            State::ReadConfig2(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self {
                        state: State::ReadConfig2(packet),
                        link_generation: self.link_generation,
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        unsafe {
                            ptr::copy_nonoverlapping(
                                response.payload.data().as_ptr(),
                                config.as_mut_ptr().add(128),
                                128,
                            );
                        }
                        if link_generation == self.link_generation {
                            // Only update the phase if we are still in the same link generation.
                            *phase = Phase::Linked {
                                frame: 0,
                                connection_failure: None,
                            };
                        }
                        None
                    }
                })
                .map_err(Error::ReadConfig2),
        }
    }
}
