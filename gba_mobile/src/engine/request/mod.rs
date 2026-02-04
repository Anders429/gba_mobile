pub(in crate::engine) mod packet;

mod error;

pub(in crate::engine) use error::Error;
pub(in crate::engine) use packet::Packet;

use crate::{
    engine::{Adapter, Source},
    mmio::serial::{self, SIOCNT, SIODATA8, SIODATA32, TransferLength},
};

const FRAMES_100_MILLISECONDS: u8 = 7;
const FRAMES_3_SECONDS: u8 = 180;

#[derive(Debug)]
pub(in crate::engine) enum Request {
    Packet(Packet),
    WaitForIdle {
        // TODO: Separate this all out into another module?
        frame: u8,
    },
}

impl Request {
    pub(in crate::engine) fn new_packet(transfer_length: TransferLength, source: Source) -> Self {
        Self::Packet(Packet::new(transfer_length, source))
    }

    pub(in crate::engine) fn new_wait_for_idle() -> Self {
        Self::WaitForIdle { frame: 0 }
    }

    pub(in crate::engine) fn vblank(&mut self, transfer_length: TransferLength) {
        match self {
            Self::Packet(_) => todo!("timeouts?"),
            Self::WaitForIdle { frame } => {
                if *frame % FRAMES_100_MILLISECONDS == 0 {
                    // Send a new idle byte.
                    match transfer_length {
                        TransferLength::_8Bit => unsafe { SIODATA8.write_volatile(0x4b) },
                        TransferLength::_32Bit => unsafe {
                            SIODATA32.write_volatile(0x4b_4b_4b_4b);
                        },
                    }
                    schedule_serial(transfer_length);
                }
                if *frame > FRAMES_3_SECONDS {
                    todo!("timeout")
                }
                *frame += 1;
            }
        }
    }

    pub(in crate::engine) fn timer(&mut self, transfer_length: TransferLength) {
        match self {
            Self::Packet(packet) => {
                packet.push();
                schedule_serial(transfer_length);
            }
            Self::WaitForIdle { .. } => {}
        }
    }

    pub(in crate::engine) fn serial(
        self,
        adapter: &mut Adapter,
        transfer_length: TransferLength,
    ) -> Result<Option<Self>, Error> {
        match self {
            Self::Packet(packet) => packet
                .pull(adapter)
                .map(|next_packet| next_packet.map(Self::Packet))
                .map_err(Error::Packet),
            Self::WaitForIdle { .. } => match transfer_length {
                TransferLength::_8Bit => {
                    if unsafe { SIODATA8.read_volatile() } == 0xd2 {
                        Ok(None)
                    } else {
                        Ok(Some(self))
                    }
                }
                TransferLength::_32Bit => {
                    if unsafe { SIODATA32.read_volatile() } == 0xd2_d2_d2_d2 {
                        Ok(None)
                    } else {
                        Ok(Some(self))
                    }
                }
            },
        }
    }
}

fn schedule_serial(transfer_length: TransferLength) {
    unsafe {
        SIOCNT.write_volatile(
            serial::Control::new()
                .master(true)
                .start(true)
                .interrupts(true)
                .transfer_length(transfer_length),
        )
    }
}
