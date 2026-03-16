mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::request::{Packet, WaitForIdle, packet::payload};
use crate::{
    Timer,
    driver::Adapter,
    mmio::serial::{SIOCNT, TransferLength},
};
use either::Either;

#[derive(Debug)]
pub(in super::super) enum End {
    EndSession(Packet<payload::EndSession>),
    WaitForIdle(WaitForIdle),
}

impl End {
    pub(super) fn new(transfer_length: TransferLength) -> Self {
        Self::EndSession(Packet::new(payload::EndSession, transfer_length))
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::EndSession(packet) => packet
                .vblank()
                .map(Self::EndSession)
                .map_err(Timeout::EndSession),
            Self::WaitForIdle(wait_for_idle) => wait_for_idle
                .vblank()
                .map(Self::WaitForIdle)
                .map_err(Timeout::WaitForIdle),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::EndSession(packet) => packet.timer(),
            Self::WaitForIdle(_) => {}
        }
    }

    pub(super) fn serial(
        self,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
    ) -> Result<Option<Self>, Error> {
        match self {
            Self::EndSession(packet) => packet
                .serial()
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::EndSession(packet)),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        *transfer_length = TransferLength::_8Bit;
                        unsafe {
                            SIOCNT.write_volatile(
                                SIOCNT.read_volatile().transfer_length(*transfer_length),
                            );
                        }
                        Some(Self::WaitForIdle(WaitForIdle::new(*transfer_length)))
                    }
                })
                .map_err(Error::EndSession),
            Self::WaitForIdle(wait_for_idle) => Ok(wait_for_idle.serial().map(Self::WaitForIdle)),
        }
    }

    pub(super) fn schedule_timer(&self, timer: Timer) {
        match self {
            Self::EndSession(packet) => packet.schedule_timer(timer),
            Self::WaitForIdle(_) => {}
        }
    }
}
