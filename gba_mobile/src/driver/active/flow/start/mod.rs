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
pub(in super::super) enum Start {
    Wake(WaitForIdle),
    BeginSession(Packet<payload::BeginSession>),
    Sio32(Packet<payload::EnableSio32>),
    WaitForIdle(WaitForIdle),
}

impl Start {
    pub(super) fn new(transfer_length: TransferLength) -> Self {
        Self::Wake(WaitForIdle::new(transfer_length))
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::Wake(wait_for_idle) => wait_for_idle
                .vblank()
                .map(Self::Wake)
                .map_err(Timeout::Wake),
            Self::BeginSession(packet) => packet
                .vblank()
                .map(Self::BeginSession)
                .map_err(Timeout::BeginSession),
            Self::Sio32(packet) => packet.vblank().map(Self::Sio32).map_err(Timeout::Sio32),
            Self::WaitForIdle(wait_for_idle) => wait_for_idle
                .vblank()
                .map(Self::WaitForIdle)
                .map_err(Timeout::WaitForIdle),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::Wake(_) => {}
            Self::BeginSession(packet) => packet.timer(),
            Self::Sio32(packet) => packet.timer(),
            Self::WaitForIdle(_) => {}
        }
    }

    pub(super) fn serial(
        self,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
        timer: Timer,
        phase: &mut Phase,
    ) -> Result<Either<Self, Response>, Error> {
        match self {
            Self::Wake(wait_for_idle) => Ok(Either::Left(wait_for_idle.serial().map_or_else(
                || Self::BeginSession(Packet::new(payload::BeginSession, *transfer_length, timer)),
                Self::Wake,
            ))),
            Self::BeginSession(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Either::Left(Self::BeginSession(packet)),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        match response.payload {
                            payload::begin_session::ReceiveParsed::BeginSession => {
                                Either::Left(Self::Sio32(Packet::new(
                                    payload::EnableSio32,
                                    *transfer_length,
                                    timer,
                                )))
                            }
                            payload::begin_session::ReceiveParsed::AlreadyActive => {
                                Either::Right(Response::AlreadyActive)
                            }
                        }
                    }
                })
                .map_err(Error::BeginSession),
            Self::Sio32(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Either::Left(Self::Sio32(packet)),
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
                        Either::Left(Self::WaitForIdle(WaitForIdle::new(*transfer_length)))
                    }
                })
                .map_err(Error::Sio32),
            Self::WaitForIdle(wait_for_idle) => Ok(wait_for_idle.serial().map_or_else(
                || {
                    *phase = Phase::Linked {
                        frame: 0,
                        connection_failure: None,
                    };
                    Either::Right(Response::Success)
                },
                |wait_for_idle| Either::Left(Self::WaitForIdle(wait_for_idle)),
            )),
        }
    }
}

#[derive(Debug)]
pub(super) enum Response {
    Success,
    AlreadyActive,
}
