use super::{
    super::{ConnectionRequest, Phase},
    request::{Packet, packet, packet::payload},
};
use crate::{Socket, Timer, driver::Adapter, mmio::serial::TransferLength, socket};
use either::Either;

#[derive(Debug)]
pub(in super::super) enum Accept {
    AcceptConnection(Packet<payload::AcceptConnection>),
}

impl Accept {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
    ) -> Self {
        Self::AcceptConnection(Packet::new(
            payload::AcceptConnection::new(packet_data),
            transfer_length,
            timer,
        ))
    }

    pub(super) fn vblank(&mut self) -> Result<(), packet::Timeout> {
        match self {
            Self::AcceptConnection(packet) => packet.vblank(),
        }
    }

    pub(super) fn timer(&mut self, packet_data: &packet::Data) {
        match self {
            Self::AcceptConnection(packet) => packet.timer(packet_data),
        }
    }

    pub(super) fn serial<Buffer>(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        packet_data: &mut packet::Data,
        phase: &mut Phase,
        socket: &mut Socket<Buffer>,
    ) -> Result<Option<Self>, packet::Error<payload::AcceptConnection>> {
        match self {
            Self::AcceptConnection(packet) => {
                packet
                    .serial(timer, packet_data)
                    .map(|response| match response {
                        Either::Left(packet) => Some(Self::AcceptConnection(packet)),
                        Either::Right(response) => {
                            *adapter = response.adapter;
                            if let Phase::Connecting(ConnectionRequest::Accept { frame, .. }) =
                                phase
                            {
                                match response.payload {
                                    payload::accept_connection::Response::Connected => {
                                        // We only update the phase if we are currently in a phase where we are
                                        // accepting connections.
                                        //
                                        // It is possible to have the phase change during execution of the
                                        // flow, in which case we should not update the phase.
                                        *phase = Phase::Connected(0);
                                        socket.id = socket::Id::P2P;
                                        socket.frame = 0;
                                    }
                                    payload::accept_connection::Response::NotConnected => {
                                        *frame = 0;
                                    }
                                }
                            }
                            None
                        }
                    })
            }
        }
    }
}
