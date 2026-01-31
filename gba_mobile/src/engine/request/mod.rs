pub(in crate::engine) mod packet;

mod error;

pub(in crate::engine) use error::Error;
pub(in crate::engine) use packet::Packet;

use crate::{
    engine::{Adapter, Source},
    mmio::serial::TransferLength,
};

#[derive(Debug)]
pub(in crate::engine) enum Request {
    Packet(Packet),
    WaitForIdle,
}

impl Request {
    pub(in crate::engine) fn new_packet(transfer_length: TransferLength, source: Source) -> Self {
        Self::Packet(Packet::new(transfer_length, source))
    }

    pub(in crate::engine) fn vblank(&mut self) {
        todo!()
    }

    pub(in crate::engine) fn timer(&mut self) {
        match self {
            Self::Packet(packet) => packet.pull(),
            Self::WaitForIdle => {}
        }
    }

    pub(in crate::engine) fn serial(self, adapter: &mut Adapter) -> Result<Option<Self>, Error> {
        match self {
            Self::Packet(packet) => packet
                .push(adapter)
                .map(|next_packet| next_packet.map(Self::Packet))
                .map_err(Error::Packet),
            Self::WaitForIdle => todo!(),
        }
    }
}
