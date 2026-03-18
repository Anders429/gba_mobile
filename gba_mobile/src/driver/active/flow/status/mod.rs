mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::{ConnectionFailure, Phase},
    request::{Packet, packet::payload},
};
use crate::{Adapter, Timer, mmio::serial::TransferLength};
use either::Either;

#[derive(Debug)]
pub(in super::super) struct Status {
    packet: Packet<payload::ConnectionStatus>,
}

impl Status {
    pub(super) fn new(transfer_length: TransferLength, timer: Timer) -> Self {
        Self {
            packet: Packet::new(payload::ConnectionStatus, transfer_length, timer),
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        self.packet
            .vblank()
            .map(|packet| Self { packet })
            .map_err(Timeout::ConnectionStatus)
    }

    pub(super) fn timer(&mut self) {
        self.packet.timer();
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        phase: &mut Phase,
    ) -> Result<Option<Self>, Error> {
        self.packet
            .serial(timer)
            .map(|response| match response {
                Either::Left(packet) => Some(Self { packet }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    match phase {
                        // Only update the phase if we are currently connected.
                        //
                        // It is possible that we could have had the phase change between when we
                        // started execution of this flow and when we completed it. In that case, we do
                        // not want to overwrite the phase.
                        Phase::Connected(frame) => {
                            match response.payload {
                                payload::connection_status::ReceiveParsed::Connected => {
                                    // Reset the frame count so that we can trigger this flow again.
                                    *frame = 0;
                                }
                                payload::connection_status::ReceiveParsed::NotConnected => {
                                    *phase = Phase::Linked {
                                        frame: 0,
                                        connection_failure: Some(ConnectionFailure::LostConnection),
                                    };
                                }
                            }
                        }
                        Phase::LoggedIn { frame, .. } => {
                            match response.payload {
                                payload::connection_status::ReceiveParsed::Connected => {
                                    // Reset the frame count so that we can trigger this flow again.
                                    *frame = 0;
                                }
                                payload::connection_status::ReceiveParsed::NotConnected => {
                                    *phase = Phase::Linked {
                                        frame: 0,
                                        connection_failure: Some(ConnectionFailure::LostConnection),
                                    };
                                }
                            }
                        }
                        _ => {}
                    }
                    None
                }
            })
            .map_err(Error::ConnectionStatus)
    }
}
