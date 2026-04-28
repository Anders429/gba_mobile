use super::request::{Packet, packet, packet::payload};
use crate::{Config, Timer, config, driver::Adapter, mmio::serial::TransferLength};
use core::{
    fmt,
    fmt::{Debug, Formatter},
    marker::PhantomData,
};
use either::Either;

pub(in super::super) struct WriteConfig<Format> {
    packet: Packet<payload::WriteConfig>,
    request: usize,
    format: PhantomData<Format>,
}

impl<Format> WriteConfig<Format>
where
    Format: config::Format,
{
    pub(super) fn new(
        transfer_length: TransferLength,
        timer: Timer,
        packet_data: &mut packet::Data,
        config: &Config<Format>,
    ) -> Option<Self> {
        if Format::WRITES == 0 {
            // There is no writing to actually be done for this config format.
            None
        } else {
            let mut data = [0; 128];
            if let config::Data::Config(format) = &config.data {
                let location = format.write(0, &mut data);
                Some(Self {
                    packet: Packet::new(
                        payload::WriteConfig::new(packet_data, location, &data),
                        transfer_length,
                        timer,
                    ),
                    request: 0,
                    format: PhantomData,
                })
            } else {
                // The config is not currently in a valid state.
                None
            }
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
        config: &Config<Format>,
    ) -> Result<Option<Self>, packet::Error<payload::WriteConfig>> {
        match self.packet.serial(timer, packet_data)? {
            Either::Left(packet) => Ok(Some(Self {
                packet,
                request: self.request,
                format: PhantomData,
            })),
            Either::Right(response) => {
                *adapter = response.adapter;
                if self.request + 1 == Format::WRITES {
                    // We are done writing.
                    Ok(None)
                } else {
                    // We still have more to write.
                    if let config::Data::Config(format) = &config.data {
                        let mut data = [0; 128];
                        let location = format.write(self.request + 1, &mut data);
                        Ok(Some(Self {
                            packet: Packet::new(
                                payload::WriteConfig::new(packet_data, location, &data),
                                transfer_length,
                                timer,
                            ),
                            request: self.request + 1,
                            format: PhantomData,
                        }))
                    } else {
                        // The config is not currently in a valid state.
                        //
                        // Note that the config could have changed between when we started this
                        // flow and now. That is acceptable; if the config changes, it is
                        // because we have received a new config write request. We will rewrite
                        // anything we have already written in this flow anyway.
                        Ok(None)
                    }
                }
            }
        }
    }
}

impl<Format> Debug for WriteConfig<Format> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("WriteConfig")
            .field("packet", &self.packet)
            .field("request", &self.request)
            .field("format", &self.format)
            .finish()
    }
}
