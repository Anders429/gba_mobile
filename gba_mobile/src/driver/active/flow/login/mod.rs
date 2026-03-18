mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::{ConnectionFailure, ConnectionRequest, Phase},
    request::{Packet, packet::payload},
};
use crate::{Adapter, ArrayVec, Digit, Generation, Timer, mmio::serial::TransferLength};
use core::net::Ipv4Addr;
use either::Either;

#[derive(Debug)]
pub(in super::super) enum Login {
    Connect {
        packet: Packet<payload::Connect>,
        login: payload::Login,
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
        adapter: Adapter,
        phone_number: ArrayVec<Digit, 32>,
        id: ArrayVec<u8, 32>,
        password: ArrayVec<u8, 32>,
        primary_dns: Ipv4Addr,
        secondary_dns: Ipv4Addr,
        connection_generation: Generation,
    ) -> Self {
        Self::Connect {
            packet: Packet::new(
                payload::Connect::new(adapter, phone_number),
                transfer_length,
                timer,
            ),
            login: payload::Login::new(id, password, primary_dns, secondary_dns),
            connection_generation,
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::Connect {
                packet,
                login,
                connection_generation,
            } => packet
                .vblank()
                .map(|packet| Self::Connect {
                    packet,
                    login,
                    connection_generation,
                })
                .map_err(Timeout::Connect),
            Self::Login {
                packet,
                connection_generation,
            } => packet
                .vblank()
                .map(|packet| Self::Login {
                    packet,
                    connection_generation,
                })
                .map_err(Timeout::Login),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::Connect { packet, .. } => packet.timer(),
            Self::Login { packet, .. } => packet.timer(),
        }
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: TransferLength,
        phase: &mut Phase,
        connection_generation: Generation,
    ) -> Result<Option<Self>, Error> {
        match self {
            Self::Connect {
                packet,
                login,
                connection_generation: flow_connection_generation,
            } => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::Connect {
                        packet,
                        login,
                        connection_generation,
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        if matches!(phase, Phase::Connecting(ConnectionRequest::Login { .. }))
                            && connection_generation == flow_connection_generation
                        {
                            // Only continue if we are currently logging in for this specific
                            // connection generation.
                            //
                            // It is possible to have the phase change during execution of the flow, in
                            // which case we should not continue or update the phase.
                            match response.payload {
                                payload::connect::ReceiveParsed::Connected => Some(Self::Login {
                                    packet: Packet::new(login, transfer_length, timer),
                                    connection_generation: flow_connection_generation,
                                }),
                                payload::connect::ReceiveParsed::NotConnected => {
                                    *phase = Phase::Linked {
                                        frame: 0,
                                        connection_failure: Some(ConnectionFailure::Connect),
                                    };
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    }
                })
                .map_err(Error::Connect),
            Self::Login {
                packet,
                connection_generation: flow_connection_generation,
            } => packet
                .serial(timer)
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
                            // Only update the phase if we are currently logging in for this specific
                            // connection generation.
                            //
                            // It is possible to have the phase change during execution of the flow, in
                            // which case we should not update the phase.
                            match response.payload.response {
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
