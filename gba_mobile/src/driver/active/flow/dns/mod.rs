mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::request::{Packet, packet::payload};
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
        name: ArrayVec<u8, MAX_LEN>,
        dns_generation: Generation,
    ) -> Self {
        Self {
            packet: Packet::new(payload::Dns::new(name), transfer_length, timer),
            dns_generation,
        }
    }

    pub(super) fn vblank(&mut self) -> Result<(), Timeout> {
        self.packet.vblank().map_err(Timeout::Dns)
    }

    pub(super) fn timer(&mut self) {
        self.packet.timer();
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        dns: &mut crate::Dns<MAX_LEN>,
    ) -> Result<Option<Self>, Error<MAX_LEN>> {
        self.packet
            .serial(timer)
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
                            payload::dns::ReceiveParsed::Success(ip) => {
                                dns.state = crate::dns::State::Success(ip);
                            }
                            payload::dns::ReceiveParsed::NotFound => {
                                dns.state = crate::dns::State::NotFound;
                            }
                        }
                    }
                    None
                }
            })
            .map_err(Error::Dns)
    }
}
