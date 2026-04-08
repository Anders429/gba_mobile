mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::Phase,
    request::{Packet, packet::payload},
};
use crate::{Generation, Socket, Timer, driver::Adapter, mmio::serial::TransferLength, socket};
use core::net::SocketAddrV4;
use either::Either;

#[derive(Debug)]
pub(in super::super) struct OpenUdp<const INDEX: usize> {
    connection_generation: Generation,
    socket_generation: Generation,
    packet: Packet<payload::OpenUdp>,
}

impl<const INDEX: usize> OpenUdp<INDEX> {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self {
            connection_generation,
            socket_generation,
            packet: Packet::new(payload::OpenUdp::new(addr), transfer_length, timer),
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        self.packet
            .vblank()
            .map(|packet| Self {
                connection_generation: self.connection_generation,
                socket_generation: self.socket_generation,
                packet,
            })
            .map_err(Timeout::OpenUdp)
    }

    pub(super) fn timer(&mut self) {
        self.packet.timer();
    }

    pub(super) fn serial<Buffer>(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        phase: &mut Phase,
        socket: &mut Socket<Buffer>,
        connection_generation: Generation,
    ) -> Result<Option<Self>, Error> {
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
            .serial(timer)
            .map(|response| match response {
                Either::Left(packet) => Some(Self {
                    connection_generation: self.connection_generation,
                    socket_generation: self.socket_generation,
                    packet,
                }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    socket.id = response.payload.id;
                    socket.frame = 0;
                    if let Some((socket_status, socket_protocol)) = socket_info {
                        *socket_status = socket::Status::Connected;
                        *socket_protocol = super::super::socket::Protocol::Udp;
                    }

                    None
                }
            })
            .map_err(Error::OpenUdp)
    }
}
