mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::{ConnectionFailure, ConnectionRequest, Phase},
    request::{Packet, packet, packet::payload},
};
use crate::{Adapter, ArrayVec, Digit, Generation, Timer, mmio::serial::TransferLength, socket};
use either::Either;

#[derive(Debug)]
pub(in super::super) enum Login {
    Connect {
        packet: Packet<payload::Connect>,
        connection_generation: Generation,
    },
    Login {
        packet: Packet<payload::Login>,
        connection_generation: Generation,
    },
}

impl Login {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
        adapter: Adapter,
        digits: &ArrayVec<Digit, 32>,
        connection_generation: Generation,
    ) -> Self {
        Self::Connect {
            packet: Packet::new(
                payload::Connect::new(packet_data, adapter, digits),
                transfer_length,
                timer,
            ),
            connection_generation,
        }
    }

    pub(super) fn vblank(&mut self) -> Result<(), Timeout> {
        match self {
            Self::Connect { packet, .. } => packet.vblank().map_err(Timeout::Connect),
            Self::Login { packet, .. } => packet.vblank().map_err(Timeout::Login),
        }
    }

    pub(super) fn timer(&mut self, packet_data: &packet::Data) {
        match self {
            Self::Connect { packet, .. } => packet.timer(packet_data),
            Self::Login { packet, .. } => packet.timer(packet_data),
        }
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        packet_data: &mut packet::Data,
        transfer_length: TransferLength,
        phase: &mut Phase,
        connection_generation: Generation,
    ) -> Result<Option<Self>, Error> {
        match self {
            Self::Connect {
                packet,
                connection_generation: flow_connection_generation,
            } => match packet.serial(timer, packet_data) {
                Ok(Either::Left(packet)) => Ok(Some(Self::Connect {
                    packet,
                    connection_generation,
                })),
                Ok(Either::Right(response)) => {
                    *adapter = response.adapter;
                    if let Phase::Connecting(ConnectionRequest::Login {
                        id,
                        password,
                        primary_dns,
                        secondary_dns,
                        ..
                    }) = phase
                        && connection_generation == flow_connection_generation
                    {
                        // Only continue if we are currently logging in for this specific
                        // connection generation.
                        //
                        // It is possible to have the phase change during execution of the flow, in
                        // which case we should not continue or update the phase.
                        match response.payload {
                            payload::connect::Response::Connected => Ok(Some(Self::Login {
                                packet: Packet::new(
                                    payload::Login::new(
                                        packet_data,
                                        id,
                                        password,
                                        *primary_dns,
                                        *secondary_dns,
                                    ),
                                    transfer_length,
                                    timer,
                                ),
                                connection_generation: flow_connection_generation,
                            })),
                            payload::connect::Response::NotConnected => {
                                *phase = Phase::Linked {
                                    frame: 0,
                                    connection_failure: Some(ConnectionFailure::Connect),
                                };
                                Ok(None)
                            }
                        }
                    } else {
                        Ok(None)
                    }
                }
                Err(error) => Err(Error::Connect(error)),
            },
            Self::Login {
                packet,
                connection_generation: flow_connection_generation,
            } => packet
                .serial(timer, packet_data)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::Login {
                        packet,
                        connection_generation,
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        if matches!(phase, Phase::Connecting(ConnectionRequest::Login { .. }))
                            && connection_generation == flow_connection_generation
                        {
                            // Only update the phase if we are currently logging in for this
                            // specific connection generation.
                            //
                            // It is possible to have the phase change during execution of the
                            // flow, in which case we should not update the phase.
                            match response.payload {
                                payload::login::Response::Connected {
                                    ip,
                                    primary_dns,
                                    secondary_dns,
                                } => {
                                    *phase = Phase::LoggedIn {
                                        frame: 0,
                                        ip,
                                        primary_dns,
                                        secondary_dns,
                                        socket_generations: [Generation::new(); 2],
                                        socket_requests: [None, None],
                                        // Arbitrarily default to TCP.
                                        //
                                        // These will be overridden when we actually connect over
                                        // the sockets.
                                        socket_protocols: [
                                            socket::Protocol::Tcp,
                                            socket::Protocol::Tcp,
                                        ],
                                    };
                                }
                                payload::login::Response::NotConnected => {
                                    *phase = Phase::Linked {
                                        frame: 0,
                                        connection_failure: Some(ConnectionFailure::Login),
                                    }
                                }
                            }
                        }
                        None
                    }
                })
                .map_err(Error::Login),
        }
    }
}
