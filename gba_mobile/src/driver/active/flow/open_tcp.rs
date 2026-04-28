use super::{
    super::Phase,
    request::{Packet, packet, packet::payload},
};
use crate::{Generation, Socket, Timer, driver::Adapter, mmio::serial::TransferLength, socket};
use core::net::SocketAddrV4;
use either::Either;

#[derive(Debug)]
pub(in super::super) struct OpenTcp<const INDEX: usize> {
    connection_generation: Generation,
    socket_generation: Generation,
    packet: Packet<payload::OpenTcp>,
}

impl<const INDEX: usize> OpenTcp<INDEX> {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self {
            connection_generation,
            socket_generation,
            packet: Packet::new(
                payload::OpenTcp::new(packet_data, addr),
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

    pub(super) fn serial<Buffer>(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        packet_data: &mut packet::Data,
        phase: &mut Phase,
        socket: &mut Socket<Buffer>,
        connection_generation: Generation,
    ) -> Result<Option<Self>, packet::Error<payload::OpenTcp>> {
        // We only should update the socket state if we are actively logged in, on the correct
        // generations, and still in a connecting state for the specific socket.
        let socket_info = if let Phase::LoggedIn {
            socket_generations,
            socket_protocols,
            ..
        } = phase
            && connection_generation == self.connection_generation
        {
            let socket_generation = unsafe { socket_generations.get_unchecked(INDEX) };
            if matches!(socket.status, socket::Status::Connecting)
                && *socket_generation == self.socket_generation
            {
                Some((&mut socket.status, &mut socket_protocols[INDEX]))
            } else {
                None
            }
        } else {
            None
        };

        self.packet
            .serial(timer, packet_data)
            .map(|response| match response {
                Either::Left(packet) => Some(Self {
                    connection_generation: self.connection_generation,
                    socket_generation: self.socket_generation,
                    packet,
                }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    match response.payload {
                        payload::open_tcp::Response::Connected(id) => {
                            socket.id = id;
                            socket.frame = 0;
                            if let Some((socket_status, socket_protocol)) = socket_info {
                                *socket_status = socket::Status::Connected;
                                *socket_protocol = socket::Protocol::Tcp;
                            }
                        }
                        payload::open_tcp::Response::NotConnected => {
                            if let Some((socket_status, _)) = socket_info {
                                *socket_status = socket::Status::FailedToConnect;
                            }
                        }
                    }
                    None
                }
            })
    }
}
