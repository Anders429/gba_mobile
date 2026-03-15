mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::{ConnectionRequest, Phase},
    request::{Packet, packet::payload},
};
use crate::{Timer, driver::Adapter, mmio::serial::TransferLength};
use either::Either;

#[derive(Debug)]
pub(super) enum Accept {
    AcceptConnection(Packet<payload::AcceptConnection>),
}

impl Accept {
    pub(super) fn new(transfer_length: TransferLength, timer: Timer) -> Self {
        Self::AcceptConnection(Packet::new(
            payload::AcceptConnection,
            transfer_length,
            timer,
        ))
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::AcceptConnection(packet) => packet
                .vblank()
                .map(Self::AcceptConnection)
                .map_err(Timeout::AcceptConnection),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::AcceptConnection(packet) => packet.timer(),
        }
    }

    pub(super) fn serial(
        self,
        adapter: &mut Adapter,
        phase: &mut Phase,
        timer: Timer,
    ) -> Result<Option<Self>, Error> {
        match self {
            Self::AcceptConnection(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::AcceptConnection(packet)),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        match response.payload {
                            payload::accept_connection::ReceiveParsed::Connected => {
                                if matches!(
                                    phase,
                                    Phase::Connecting(ConnectionRequest::Accept { .. })
                                ) {
                                    // We only update the phase if we are currently in a phase where we are
                                    // accepting connections.
                                    //
                                    // It is possible to have the phase change during execution of the
                                    // flow, in which case we should not update the phase.
                                    *phase = Phase::Connected;
                                }
                            }
                            payload::accept_connection::ReceiveParsed::NotConnected => {}
                        };
                        None
                    }
                })
                .map_err(Error::AcceptConnection),
        }
    }
}
