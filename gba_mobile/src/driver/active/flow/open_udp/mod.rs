mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::Phase,
    request::{Packet, packet::payload},
};
use crate::{
    ArrayVec, Generation, Socket, Timer, driver::Adapter, mmio::serial::TransferLength, socket,
};
use core::net::SocketAddrV4;
use either::Either;

#[derive(Debug)]
enum State {
    Dns {
        packet: Packet<payload::Dns>,
        port: u16,
    },
    OpenUdp(Packet<payload::OpenUdp>),
}

#[derive(Debug)]
pub(in super::super) struct OpenUdp<const INDEX: usize> {
    connection_generation: Generation,
    socket_generation: Generation,
    state: State,
}

impl<const INDEX: usize> OpenUdp<INDEX> {
    pub(super) fn with_dns(
        transfer_length: TransferLength,
        timer: Timer,
        domain: ArrayVec<u8, 255>,
        port: u16,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self {
            connection_generation,
            socket_generation,
            state: State::Dns {
                packet: Packet::new(payload::Dns::new(domain), transfer_length, timer),
                port,
            },
        }
    }

    pub(super) fn with_socket_addr(
        transfer_length: TransferLength,
        timer: Timer,
        addr: SocketAddrV4,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self {
            connection_generation,
            socket_generation,
            state: State::OpenUdp(Packet::new(
                payload::OpenUdp::new(addr),
                transfer_length,
                timer,
            )),
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self.state {
            State::Dns { packet, port } => packet
                .vblank()
                .map(|packet| Self {
                    connection_generation: self.connection_generation,
                    socket_generation: self.socket_generation,
                    state: State::Dns { packet, port },
                })
                .map_err(Timeout::Dns),
            State::OpenUdp(packet) => packet
                .vblank()
                .map(|packet| Self {
                    connection_generation: self.connection_generation,
                    socket_generation: self.socket_generation,
                    state: State::OpenUdp(packet),
                })
                .map_err(Timeout::OpenUdp),
        }
    }

    pub(super) fn timer(&mut self) {
        match &mut self.state {
            State::Dns { packet, .. } => packet.timer(),
            State::OpenUdp(packet) => packet.timer(),
        }
    }

    pub(super) fn serial<Buffer>(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: TransferLength,
        phase: &mut Phase,
        socket: &mut Socket<Buffer>,
        connection_generation: Generation,
    ) -> Result<Option<Self>, Error> {
        // We only should update the socket state if we are actively logged in, on the correct
        // generations, and still in a connecting state for the specific socket.
        let socket_status = if let Phase::LoggedIn {
            socket_generations, ..
        } = phase
            && connection_generation == self.connection_generation
        {
            let socket_generation = unsafe { socket_generations.get_unchecked(INDEX) };
            if matches!(socket.status, socket::Status::Connecting)
                && *socket_generation == self.socket_generation
            {
                Some(&mut socket.status)
            } else {
                None
            }
        } else {
            None
        };

        match self.state {
            State::Dns { packet, port } => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self {
                        connection_generation: self.connection_generation,
                        socket_generation: self.socket_generation,
                        state: State::Dns { packet, port },
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        match response.payload {
                            payload::dns::ReceiveParsed::Success(ip) => Some(Self {
                                connection_generation: self.connection_generation,
                                socket_generation: self.socket_generation,
                                state: State::OpenUdp(Packet::new(
                                    payload::OpenUdp::new(SocketAddrV4::new(ip, port)),
                                    transfer_length,
                                    timer,
                                )),
                            }),
                            payload::dns::ReceiveParsed::NotFound => {
                                if let Some(socket_status) = socket_status {
                                    *socket_status = socket::Status::ConnectionFailure;
                                }
                                None
                            }
                        }
                    }
                })
                .map_err(Error::Dns),
            State::OpenUdp(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self {
                        connection_generation: self.connection_generation,
                        socket_generation: self.socket_generation,
                        state: State::OpenUdp(packet),
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        socket.id = response.payload.id;
                        socket.frame = 0;
                        if let Some(socket_status) = socket_status {
                            *socket_status = socket::Status::Connected;
                        }

                        None
                    }
                })
                .map_err(Error::OpenUdp),
        }
    }
}
