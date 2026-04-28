use super::request::{Packet, packet, packet::payload};
use crate::{ArrayVec, Generation, Timer, driver::Adapter, mmio::serial::TransferLength};
use either::Either;

#[derive(Debug)]
pub(in super::super) struct Dns<const MAX_LEN: usize> {
    packet: Packet<payload::Dns<MAX_LEN>>,
    dns_generation: Generation,
}

impl<const MAX_LEN: usize> Dns<MAX_LEN> {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
        name: ArrayVec<u8, MAX_LEN>,
        dns_generation: Generation,
    ) -> Self {
        Self {
            packet: Packet::new(
                payload::Dns::new(packet_data, &name),
                transfer_length,
                timer,
            ),
            dns_generation,
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
        dns: &mut crate::Dns<MAX_LEN>,
    ) -> Result<Option<Self>, packet::Error<payload::Dns<MAX_LEN>>> {
        self.packet
            .serial(timer, packet_data)
            .map(|response| match response {
                Either::Left(packet) => Some(Self {
                    packet,
                    dns_generation: self.dns_generation,
                }),
                Either::Right(response) => {
                    *adapter = response.adapter;
                    // Only update state if this is the same DNS request.
                    if dns.generation == self.dns_generation
                        && matches!(dns.state, crate::dns::State::Request(_))
                    {
                        match response.payload {
                            payload::dns::Response::Success(ip) => {
                                dns.state = crate::dns::State::Success(ip);
                            }
                            payload::dns::Response::NotFound => {
                                dns.state = crate::dns::State::NotFound;
                            }
                        }
                    }
                    None
                }
            })
    }
}
