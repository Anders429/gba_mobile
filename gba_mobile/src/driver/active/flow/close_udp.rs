use super::{
    super::{ConnectionFailure, Phase},
    request::{Packet, packet, packet::payload},
};
use crate::{Timer, driver::Adapter, mmio::serial::TransferLength, socket};
use either::Either;

#[derive(Debug)]
pub(in super::super) struct CloseUdp {
    packet: Packet<payload::CloseUdp>,
}

impl CloseUdp {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
        id: socket::Id,
    ) -> Self {
        Self {
            packet: Packet::new(
                payload::CloseUdp::new(packet_data, id),
                transfer_length,
                timer,
            ),
        }
    }

    pub(super) fn vblank(&mut self) -> Result<(), packet::Timeout> {
        self.packet.vblank()
    }

    pub(super) fn timer(&mut self, packet_data: &packet::Data) {
        self.packet.timer(packet_data);
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        packet_data: &mut packet::Data,
        phase: &mut Phase,
    ) -> Result<Option<Self>, packet::Error<payload::CloseUdp>> {
        self.packet
            .serial(timer, packet_data)
            .map(|response| match response {
                Either::Left(packet) => Some(Self { packet }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    if matches!(
                        response.payload,
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
    }
}
