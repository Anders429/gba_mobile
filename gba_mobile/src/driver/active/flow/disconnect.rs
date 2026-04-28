use super::request::{Packet, packet, packet::payload};
use crate::{Timer, driver::Adapter, mmio::serial::TransferLength};
use either::Either;

#[derive(Debug)]
pub(in super::super) struct Disconnect {
    packet: Packet<payload::Disconnect>,
}

impl Disconnect {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
    ) -> Self {
        Self {
            packet: Packet::new(
                payload::Disconnect::new(packet_data),
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
    ) -> Result<Option<Self>, packet::Error<payload::Disconnect>> {
        self.packet
            .serial(timer, packet_data)
            .map(|response| match response {
                Either::Left(packet) => Some(Self { packet }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    None
                }
            })
    }
}
