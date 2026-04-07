mod error;
mod timeout;

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
use core::ptr;
use either::Either;

#[derive(Debug)]
enum State {
    Wake(WaitForIdle),
    BeginSession(Packet<payload::BeginSession>),
    Sio32(Packet<payload::EnableSio32>),
    WaitForIdle(WaitForIdle),
    ReadConfig1(Packet<payload::ReadConfig>),
    ReadConfig2(Packet<payload::ReadConfig>),
}

#[derive(Debug)]
pub(in super::super) struct Start {
    state: State,
    link_generation: Generation,
}

impl Start {
    pub(super) fn new(transfer_length: TransferLength, link_generation: Generation) -> Self {
        Self {
            state: State::Wake(WaitForIdle::new(transfer_length)),
            link_generation,
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self.state {
            State::Wake(wait_for_idle) => wait_for_idle
                .vblank()
                .map(|wait_for_idle| Self {
                    state: State::Wake(wait_for_idle),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::Wake),
            State::BeginSession(packet) => packet
                .vblank()
                .map(|packet| Self {
                    state: State::BeginSession(packet),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::BeginSession),
            State::Sio32(packet) => packet
                .vblank()
                .map(|packet| Self {
                    state: State::Sio32(packet),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::Sio32),
            State::WaitForIdle(wait_for_idle) => wait_for_idle
                .vblank()
                .map(|wait_for_idle| Self {
                    state: State::WaitForIdle(wait_for_idle),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::WaitForIdle),
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
            State::Wake(_) => {}
            State::BeginSession(packet) => packet.timer(),
            State::Sio32(packet) => packet.timer(),
            State::WaitForIdle(_) => {}
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
    ) -> Result<Either<Self, Response>, Error> {
        match self.state {
            State::Wake(wait_for_idle) => Ok(Either::Left(wait_for_idle.serial().map_or_else(
                || Self {
                    state: State::BeginSession(Packet::new(
                        payload::BeginSession,
                        *transfer_length,
                        timer,
                    )),
                    link_generation: self.link_generation,
                },
                |wait_for_idle| Self {
                    state: State::Wake(wait_for_idle),
                    link_generation: self.link_generation,
                },
            ))),
            State::BeginSession(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Either::Left(Self {
                        state: State::BeginSession(packet),
                        link_generation: self.link_generation,
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        match response.payload {
                            payload::begin_session::ReceiveParsed::BeginSession => {
                                Either::Left(Self {
                                    state: State::Sio32(Packet::new(
                                        payload::EnableSio32,
                                        *transfer_length,
                                        timer,
                                    )),
                                    link_generation: self.link_generation,
                                })
                            }
                            payload::begin_session::ReceiveParsed::AlreadyActive => {
                                Either::Right(Response::AlreadyActive)
                            }
                        }
                    }
                })
                .map_err(Error::BeginSession),
            State::Sio32(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Either::Left(Self {
                        state: State::Sio32(packet),
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
                            state: State::WaitForIdle(WaitForIdle::new(*transfer_length)),
                            link_generation: self.link_generation,
                        })
                    }
                })
                .map_err(Error::Sio32),
            State::WaitForIdle(wait_for_idle) => {
                Ok(Either::Left(wait_for_idle.serial().map_or_else(
                    || Self {
                        state: State::ReadConfig1(Packet::new(
                            payload::ReadConfig::FirstHalf,
                            *transfer_length,
                            timer,
                        )),
                        link_generation: self.link_generation,
                    },
                    |wait_for_idle| Self {
                        state: State::WaitForIdle(wait_for_idle),
                        link_generation: self.link_generation,
                    },
                )))
            }
            State::ReadConfig1(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Either::Left(Self {
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
                        Either::Left(Self {
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
                    Either::Left(packet) => Either::Left(Self {
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
                        Either::Right(Response::Success)
                    }
                })
                .map_err(Error::ReadConfig2),
        }
    }
}

#[derive(Debug)]
pub(super) enum Response {
    Success,
    AlreadyActive,
}
