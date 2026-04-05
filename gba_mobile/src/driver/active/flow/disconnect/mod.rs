mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::Phase,
    request::{Packet, packet::payload},
};
use crate::{Timer, driver::Adapter, mmio::serial::TransferLength};
use either::Either;

#[derive(Debug)]
pub(in super::super) struct Disconnect {
    packet: Packet<payload::Disconnect>,
}

impl Disconnect {
    pub(super) fn new(transfer_length: TransferLength, timer: Timer) -> Self {
        Self {
            packet: Packet::new(payload::Disconnect, transfer_length, timer),
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        self.packet
            .vblank()
            .map(|packet| Self { packet })
            .map_err(Timeout::Disconnect)
    }

    pub(super) fn timer(&mut self) {
        self.packet.timer();
    }

    pub(super) fn serial(self, timer: Timer, adapter: &mut Adapter) -> Result<Option<Self>, Error> {
        self.packet
            .serial(timer)
            .map(|response| match response {
                Either::Left(packet) => Some(Self { packet }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    None
                }
            })
            .map_err(Error::Disconnect)
    }
}
