use super::{
    super::{ConnectionFailure, Phase},
    request::{Packet, packet, packet::payload},
};
use crate::{Adapter, Timer, mmio::serial::TransferLength};
use either::Either;

#[derive(Debug)]
pub(in super::super) struct Status {
    packet: Packet<payload::ConnectionStatus>,
}

impl Status {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
    ) -> Self {
        Self {
            packet: Packet::new(
                payload::ConnectionStatus::new(packet_data),
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

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        packet_data: &mut packet::Data,
        phase: &mut Phase,
    ) -> Result<Option<Self>, packet::Error<payload::ConnectionStatus>> {
        self.packet
            .serial(timer, packet_data)
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
                                payload::connection_status::Response::Connected => {
                                    // Reset the frame count so that we can trigger this flow again.
                                    *frame = 0;
                                }
                                payload::connection_status::Response::NotConnected => {
                                    *phase = Phase::Linked {
                                        frame: 0,
                                        connection_failure: Some(ConnectionFailure::LostConnection),
                                    };
                                }
                            }
                        }
                        Phase::LoggedIn { frame, .. } => {
                            match response.payload {
                                payload::connection_status::Response::Connected => {
                                    // Reset the frame count so that we can trigger this flow again.
                                    *frame = 0;
                                }
                                payload::connection_status::Response::NotConnected => {
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
    }
}
