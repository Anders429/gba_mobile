mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::request::{Packet, WaitForIdle, packet::payload};
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

    pub(super) fn vblank(&mut self) -> Result<(), Timeout> {
        match &mut self.state {
            State::Reset(packet) => packet.vblank().map_err(Timeout::Reset),
            State::WaitForSio8(wait_for_idle) => {
                wait_for_idle.vblank().map_err(Timeout::WaitForSio8)
            }
            State::EnableSio32(packet) => packet.vblank().map_err(Timeout::EnableSio32),
            State::WaitForSio32(wait_for_idle) => {
                wait_for_idle.vblank().map_err(Timeout::WaitForSio32)
            }
        }
    }

    pub(super) fn timer(&mut self) {
        match &mut self.state {
            State::Reset(packet) => packet.timer(),
            State::WaitForSio8(_) => {}
            State::EnableSio32(packet) => packet.timer(),
            State::WaitForSio32(_) => {}
        }
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
        link_generation: Generation,
    ) -> Result<Either<Self, Response>, Error> {
        match self.state {
            State::Reset(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Either::Left(Self {
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
                        Either::Left(Self {
                            state: State::WaitForSio8(WaitForIdle::new(*transfer_length)),
                            link_generation: self.link_generation,
                        })
                    }
                })
                .map_err(Error::Reset),
            State::WaitForSio8(wait_for_idle) => {
                Ok(Either::Left(wait_for_idle.serial().map_or_else(
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
                )))
            }
            State::EnableSio32(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Either::Left(Self {
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
                        Either::Left(Self {
                            state: State::WaitForSio32(WaitForIdle::new(*transfer_length)),
                            link_generation: self.link_generation,
                        })
                    }
                })
                .map_err(Error::EnableSio32),
            State::WaitForSio32(wait_for_idle) => Ok(wait_for_idle.serial().map_or_else(
                || {
                    if link_generation == self.link_generation {
                        Either::Right(Response::Success)
                    } else {
                        Either::Right(Response::Superseded)
                    }
                },
                |wait_for_idle| {
                    Either::Left(Self {
                        state: State::WaitForSio32(wait_for_idle),
                        link_generation: self.link_generation,
                    })
                },
            )),
        }
    }
}

#[derive(Debug)]
pub(super) enum Response {
    Success,
    Superseded,
}
