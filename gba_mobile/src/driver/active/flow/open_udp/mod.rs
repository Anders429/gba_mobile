mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::{Phase, Socket, socket},
    request::{Packet, packet::payload},
};
use crate::{ArrayVec, Generation, Timer, driver::Adapter, mmio::serial::TransferLength};
use core::net::SocketAddrV4;
use deranged::RangedU8;
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
pub(in super::super) struct OpenUdp {
    socket_index: RangedU8<0, 1>,
    connection_generation: Generation,
    socket_generation: Generation,
    state: State,
}

impl OpenUdp {
    pub(super) fn with_dns(
        transfer_length: TransferLength,
        timer: Timer,
        domain: ArrayVec<u8, 255>,
        port: u16,
        socket_index: RangedU8<0, 1>,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self {
            socket_index,
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
        socket_index: RangedU8<0, 1>,
        connection_generation: Generation,
        socket_generation: Generation,
    ) -> Self {
        Self {
            socket_index,
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
                    socket_index: self.socket_index,
                    connection_generation: self.connection_generation,
                    socket_generation: self.socket_generation,
                    state: State::Dns { packet, port },
                })
                .map_err(Timeout::Dns),
            State::OpenUdp(packet) => packet
                .vblank()
                .map(|packet| Self {
                    socket_index: self.socket_index,
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

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: TransferLength,
        phase: &mut Phase,
        sockets: &mut [Socket; 2],
        connection_generation: Generation,
    ) -> Result<Option<Self>, Error> {
        // We only should update the socket state if we are actively logged in, on the correct
        // generations, and still in a connecting state for the specific socket.
        let socket_state = if let Phase::LoggedIn {
            socket_generations,
            socket_states,
            ..
        } = phase
            && connection_generation == self.connection_generation
        {
            let socket_generation =
                unsafe { socket_generations.get_unchecked(self.socket_index.get() as usize) };
            let socket_state =
                unsafe { socket_states.get_unchecked_mut(self.socket_index.get() as usize) };
            if matches!(socket_state, socket::State::Connecting(_, _))
                && *socket_generation == self.socket_generation
            {
                Some(socket_state)
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
                        socket_index: self.socket_index,
                        connection_generation: self.connection_generation,
                        socket_generation: self.socket_generation,
                        state: State::Dns { packet, port },
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        match response.payload {
                            payload::dns::ReceiveParsed::Success(ip) => Some(Self {
                                socket_index: self.socket_index,
                                connection_generation: self.connection_generation,
                                socket_generation: self.socket_generation,
                                state: State::OpenUdp(Packet::new(
                                    payload::OpenUdp::new(SocketAddrV4::new(ip, port)),
                                    transfer_length,
                                    timer,
                                )),
                            }),
                            payload::dns::ReceiveParsed::NotFound => {
                                if let Some(socket_state) = socket_state {
                                    *socket_state = socket::State::Failure(socket::Failure::Dns);
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
                        socket_index: self.socket_index,
                        connection_generation: self.connection_generation,
                        socket_generation: self.socket_generation,
                        state: State::OpenUdp(packet),
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        let socket =
                            unsafe { sockets.get_unchecked_mut(self.socket_index.get() as usize) };
                        socket.set_id(response.payload.id);
                        if let Some(socket_state) = socket_state {
                            *socket_state = socket::State::Connected;
                        }

                        None
                    }
                })
                .map_err(Error::OpenUdp),
        }
    }
}
