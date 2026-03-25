mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::{ConnectionFailure, ConnectionRequest, Phase, Socket, socket},
    request::{Packet, packet::payload},
};
use crate::{ArrayVec, Digit, Generation, Timer, driver::Adapter, mmio::serial::TransferLength};
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
        adapter: Adapter,
        phone_number: ArrayVec<Digit, 32>,
        connection_generation: Generation,
    ) -> Self {
        Self {
            packet: Packet::new(
                payload::Connect::new(adapter, phone_number),
                transfer_length,
                timer,
            ),
            connection_generation,
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        self.packet
            .vblank()
            .map(|packet| Self {
                packet,
                connection_generation: self.connection_generation,
            })
            .map_err(Timeout::Connect)
    }

    pub(super) fn timer(&mut self) {
        self.packet.timer()
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        phase: &mut Phase,
        socket: &mut Socket,
        connection_generation: Generation,
    ) -> Result<Option<Self>, Error> {
        self.packet
            .serial(timer)
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
                            payload::connect::ReceiveParsed::Connected => {
                                *phase = Phase::Connected(0);
                                socket.set_id(socket::Id::P2P);
                            }
                            payload::connect::ReceiveParsed::NotConnected => {
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
            .map_err(Error::Connect)
    }
}
