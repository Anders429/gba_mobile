mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::{ConnectionFailure, Phase},
    request::{Packet, packet::payload},
};
use crate::{Timer, driver::Adapter, mmio::serial::TransferLength, socket};
use either::Either;

#[derive(Debug)]
pub(in super::super) struct CloseUdp {
    packet: Packet<payload::CloseUdp>,
}

impl CloseUdp {
    pub(super) fn new(transfer_length: TransferLength, timer: Timer, id: socket::Id) -> Self {
        Self {
            packet: Packet::new(payload::CloseUdp::new(id), transfer_length, timer),
        }
    }

    pub(super) fn vblank(&mut self) -> Result<(), Timeout> {
        self.packet.vblank().map_err(Timeout::CloseUdp)
    }

    pub(super) fn timer(&mut self) {
        self.packet.timer();
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        phase: &mut Phase,
    ) -> Result<Option<Self>, Error> {
        self.packet
            .serial(timer)
            .map(|response| match response {
                Either::Left(packet) => Some(Self { packet }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    if matches!(
                        response.payload.response,
                        payload::close_udp::Response::AlreadyDisconnected
                    ) {
                        // If we get this response, we are no longer connected to the internet.
                        *phase = Phase::Linked {
                            frame: 0,
                            connection_failure: Some(ConnectionFailure::LostConnection),
                        }
                    }
                    None
                }
            })
            .map_err(Error::CloseUdp)
    }
}
