mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::{Phase, Socket, socket},
    request::{Packet, packet::payload},
};
use crate::{ArrayVec, Timer, driver::Adapter, mmio::serial::TransferLength};
use either::Either;

#[derive(Debug)]
pub(in super::super) struct TransferData {
    packet: Packet<payload::TransferData>,
    index: crate::socket::Index,
}

impl TransferData {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        id: socket::Id,
        data: ArrayVec<u8, 254>,
        index: crate::socket::Index,
    ) -> Self {
        Self {
            packet: Packet::new(payload::TransferData::new(id, data), transfer_length, timer),
            index,
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        self.packet
            .vblank()
            .map(|packet| Self {
                packet,
                index: self.index,
            })
            .map_err(Timeout::TransferData)
    }

    pub(super) fn timer(&mut self) {
        self.packet.timer();
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        sockets: &mut [Socket; 2],
        phase: &mut Phase,
    ) -> Result<Option<Self>, Error> {
        self.packet
            .serial(timer)
            .map(|response| match response {
                Either::Left(packet) => Some(Self {
                    packet,
                    index: self.index,
                }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    let socket_state = if let Phase::LoggedIn { socket_states, .. } = phase {
                        Some(&mut socket_states[usize::from(self.index)])
                    } else {
                        None
                    };
                    match response.payload.response {
                        // TODO: Store the data.
                        payload::transfer_data::Response::Data(data) => {
                            sockets[usize::from(self.index)].reset_frame();
                        }
                        payload::transfer_data::Response::FinalData(data) => {
                            sockets[usize::from(self.index)].reset_frame();
                        }
                        payload::transfer_data::Response::ConnectionFailed => {
                            if let Some(socket_state) = socket_state {
                                *socket_state =
                                    socket::State::Failure(socket::Failure::ConnectionFailed);
                            }
                        }
                    }
                    None
                }
            })
            .map_err(Error::TransferData)
    }
}
