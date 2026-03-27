mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::request::{Packet, packet::payload};
use crate::{Timer, driver::Adapter, mmio::serial::TransferLength};
use core::ptr;
use either::Either;

#[derive(Debug)]
pub(in super::super) enum WriteConfig {
    WriteConfig1(Packet<payload::WriteConfig>, [u8; 128]),
    WriteConfig2(Packet<payload::WriteConfig>),
}

impl WriteConfig {
    pub(super) fn new(transfer_length: TransferLength, timer: Timer, config: &[u8; 256]) -> Self {
        let mut first_half = [0; 128];
        let mut second_half = [0; 128];
        unsafe {
            ptr::copy_nonoverlapping(config.as_ptr(), first_half.as_mut_ptr(), 128);
            ptr::copy_nonoverlapping(config.as_ptr().add(128), second_half.as_mut_ptr(), 128);
        }

        Self::WriteConfig1(
            Packet::new(
                payload::WriteConfig::new(payload::write_config::Location::FirstHalf, first_half),
                transfer_length,
                timer,
            ),
            second_half,
        )
    }

    pub(super) fn vblank(self) -> Result<Self, Timeout> {
        match self {
            Self::WriteConfig1(packet, data) => packet
                .vblank()
                .map(|packet| Self::WriteConfig1(packet, data))
                .map_err(Timeout::WriteConfig1),
            Self::WriteConfig2(packet) => packet
                .vblank()
                .map(Self::WriteConfig2)
                .map_err(Timeout::WriteConfig2),
        }
    }

    pub(super) fn timer(&mut self) {
        match self {
            Self::WriteConfig1(packet, _) => packet.timer(),
            Self::WriteConfig2(packet) => packet.timer(),
        }
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        transfer_length: TransferLength,
    ) -> Result<Option<Self>, Error> {
        match self {
            Self::WriteConfig1(packet, data) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::WriteConfig1(packet, data)),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        Some(Self::WriteConfig2(Packet::new(
                            payload::WriteConfig::new(
                                payload::write_config::Location::SecondHalf,
                                data,
                            ),
                            transfer_length,
                            timer,
                        )))
                    }
                })
                .map_err(Error::WriteConfig1),
            Self::WriteConfig2(packet) => packet
                .serial(timer)
                .map(|response| match response {
                    Either::Left(packet) => Some(Self::WriteConfig2(packet)),
                    Either::Right(response) => {
                        *adapter = response.adapter;
                        None
                    }
                })
                .map_err(Error::WriteConfig2),
        }
    }
}
