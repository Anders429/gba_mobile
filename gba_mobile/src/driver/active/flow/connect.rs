use super::{
    super::{ConnectionFailure, ConnectionRequest, Phase},
    request::{Packet, packet, packet::payload},
};
use crate::{
    ArrayVec, Digit, Generation, Socket, Timer, driver::Adapter, mmio::serial::TransferLength,
    socket,
};
use either::Either;

#[derive(Debug)]
pub(in super::super) struct Connect {
    packet: Packet<payload::Connect>,
    connection_generation: Generation,
}

impl Connect {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
        adapter: Adapter,
        digits: &ArrayVec<Digit, 32>,
        connection_generation: Generation,
    ) -> Self {
        Self {
            packet: Packet::new(
                payload::Connect::new(packet_data, adapter, digits),
                transfer_length,
                timer,
            ),
            connection_generation,
        }
    }

    pub(super) fn vblank(&mut self) -> Result<(), packet::Timeout> {
        self.packet.vblank()
    }

    pub(super) fn timer(&mut self, packet_data: &packet::Data) {
        self.packet.timer(packet_data)
    }

    pub(super) fn serial<Buffer>(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        packet_data: &mut packet::Data,
        phase: &mut Phase,
        socket: &mut Socket<Buffer>,
        connection_generation: Generation,
    ) -> Result<Option<Self>, packet::Error<payload::Connect>> {
        self.packet
            .serial(timer, packet_data)
            .map(|response| match response {
                Either::Left(packet) => Some(Self {
                    packet,
                    connection_generation: self.connection_generation,
                }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    if matches!(phase, Phase::Connecting(ConnectionRequest::Connect { .. }))
                        && connection_generation == self.connection_generation
                    {
                        // Only update the phase if we are currently in the phase of connecting for
                        // this specific connection generation.
                        //
                        // It is possible to have the phase change during execution of the flow, in
                        // which case we should not update the phase.
                        match response.payload {
                            payload::connect::Response::Connected => {
                                *phase = Phase::Connected(0);
                                socket.id = socket::Id::P2P;
                                socket.frame = 0;
                            }
                            payload::connect::Response::NotConnected => {
                                *phase = Phase::Linked {
                                    frame: 0,
                                    connection_failure: Some(ConnectionFailure::Connect),
                                }
                            }
                        };
                    }
                    None
                }
            })
    }
}
