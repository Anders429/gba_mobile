use super::{
    super::Phase,
    request::{Packet, packet, packet::payload},
};
use crate::{
    Config, Generation, Timer, config, config::format::Segments, driver::Adapter,
    mmio::serial::TransferLength,
};
use core::{
    fmt,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    mem,
};
use either::Either;

pub(in super::super) struct ReadConfig<Format> {
    packet: Packet<payload::ReadConfig>,
    link_generation: Generation,
    format: PhantomData<Format>,
}

impl<Format> ReadConfig<Format>
where
    Format: config::Format,
{
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
        link_generation: Generation,
        config: &Config<Format>,
    ) -> Option<Self> {
        if let config::Data::Segments(segments) = &config.data {
            Some(Self {
                packet: Packet::new(
                    payload::ReadConfig::new(packet_data, segments.location()),
                    transfer_length,
                    timer,
                ),
                link_generation,
                format: PhantomData,
            })
        } else {
            // The config is already read.
            None
        }
    }

    pub(super) fn vblank(&mut self) -> Result<(), packet::Timeout> {
        self.packet.vblank()
    }

    pub(super) fn timer(&mut self, packet_data: &packet::Data) {
        self.packet.timer(packet_data)
    }

    pub(super) fn serial(
        self,
        timer: Timer,
        adapter: &mut Adapter,
        packet_data: &mut packet::Data,
        transfer_length: TransferLength,
        config: &mut Config<Format>,
        phase: &mut Phase,
        link_generation: Generation,
    ) -> Result<Option<Self>, packet::Error<payload::ReadConfig>> {
        match self.packet.serial(timer, packet_data)? {
            Either::Left(packet) => Ok(Some(Self {
                packet,
                link_generation: self.link_generation,
                format: PhantomData,
            })),
            Either::Right(response) => {
                *adapter = response.adapter;
                if let config::Data::Segments(segments) = &mut config.data {
                    let result =
                        match mem::replace(segments, Format::segments()).read(response.payload) {
                            Ok(config::format::ReadResult::Segments(segments)) => {
                                // Store this state and continue reading the next segment.
                                let location = segments.location();
                                config.data = config::Data::Segments(segments);
                                Some(Self {
                                    packet: Packet::new(
                                        payload::ReadConfig::new(packet_data, location),
                                        transfer_length,
                                        timer,
                                    ),
                                    link_generation: self.link_generation,
                                    format: PhantomData,
                                })
                            }
                            Ok(config::format::ReadResult::Success(format)) => {
                                // Store the data and finish reading.
                                config.data = config::Data::Config(format);
                                None
                            }
                            Err(error) => {
                                // Store the error and finish reading.
                                config.data = config::Data::Error(error);
                                None
                            }
                        };
                    if result.is_none() {
                        if link_generation == self.link_generation {
                            // If we are still in the same link generation, update the phase to
                            // indicate that we are fully linked.
                            *phase = Phase::Linked {
                                frame: 0,
                                connection_failure: None,
                            };
                        }
                    }
                    Ok(result)
                } else {
                    // We've already either finished reading the config or encountered an error. We just finish reading.
                    Ok(None)
                }
            }
        }
    }
}

impl<Format> Debug for ReadConfig<Format> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("ReadConfig")
            .field("packet", &self.packet)
            .field("link_generation", &self.link_generation)
            .field("format", &self.format)
            .finish()
    }
}
