mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{
    super::Phase,
    request::{Packet, packet::payload},
};
use crate::{Config, Generation, Timer, config, driver::Adapter, mmio::serial::TransferLength};
use core::{mem::MaybeUninit, ptr};
use either::Either;

#[derive(Debug)]
enum State {
    ReadConfig1(Packet<payload::ReadConfig>),
    ReadConfig2([u8; 128], Packet<payload::ReadConfig>),
}

#[derive(Debug)]
pub(in super::super) struct ReadConfig {
    state: State,
    link_generation: Generation,
}

impl ReadConfig {
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        link_generation: Generation,
    ) -> Self {
        Self {
            state: State::ReadConfig1(Packet::new(
                payload::ReadConfig::FirstHalf,
                transfer_length,
                timer,
            )),
            link_generation,
        }
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self.state {
            State::ReadConfig1(packet) => packet
                .vblank()
                .map(|packet| Self {
                    state: State::ReadConfig1(packet),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::ReadConfig1),
            State::ReadConfig2(first_half, packet) => packet
                .vblank()
                .map(|packet| Self {
                    state: State::ReadConfig2(first_half, packet),
                    link_generation: self.link_generation,
                })
                .map_err(Timeout::ReadConfig2),
        }
    }

    pub(super) fn timer(&mut self) {
        match &mut self.state {
            State::ReadConfig1(packet) => packet.timer(),
            State::ReadConfig2(_, packet) => packet.timer(),
        }
    }

    pub(super) fn serial<Format>(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: TransferLength,
        config: &mut Config<Format>,
        phase: &mut Phase,
        link_generation: Generation,
    ) -> Result<Option<Self>, Error>
    where
        Format: config::Format,
    {
        match self.state {
            State::ReadConfig1(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self {
                        state: State::ReadConfig1(packet),
                        link_generation: self.link_generation,
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        Some(Self {
                            state: State::ReadConfig2(
                                response.payload.data(),
                                Packet::new(
                                    payload::ReadConfig::SecondHalf,
                                    transfer_length,
                                    timer,
                                ),
                            ),
                            link_generation: self.link_generation,
                        })
                    }
                })
                .map_err(Error::ReadConfig1),
            State::ReadConfig2(first_half, packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self {
                        state: State::ReadConfig2(first_half, packet),
                        link_generation: self.link_generation,
                    }),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        let mut full_config = [0; 256];
                        unsafe {
                            ptr::copy_nonoverlapping(
                                first_half.as_ptr(),
                                full_config.as_mut_ptr(),
                                128,
                            );
                            ptr::copy_nonoverlapping(
                                response.payload.data().as_ptr(),
                                full_config.as_mut_ptr().add(128),
                                128,
                            );
                        }
                        let result = Format::read(&full_config);
                        config.data = MaybeUninit::new(result);

                        if link_generation == self.link_generation {
                            // If we are still in the same link generation, update the phase to
                            // indicate that we are fully linked.
                            *phase = Phase::Linked {
                                frame: 0,
                                connection_failure: None,
                            };
                        }

                        None
                    }
                })
                .map_err(Error::ReadConfig2),
        }
    }
}
