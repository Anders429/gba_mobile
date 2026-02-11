pub(in crate::driver) mod packet;

mod error;
mod timeout;

use either::Either;
pub(in crate::driver) use error::Error;
pub(in crate::driver) use packet::Packet;
pub(in crate::driver) use timeout::Timeout;

use crate::{
    Timer,
    driver::{Adapter, Source, command, frames},
    mmio::{
        serial::{self, SIOCNT, SIODATA8, SIODATA32, TransferLength},
        timer::{self, TM0CNT, TM0VAL, TM1CNT, TM1VAL, TM2CNT, TM2VAL, TM3CNT, TM3VAL},
    },
};

// These are at a rate of ~60us per tick.
const TIMER_200_MICROSECONDS: u16 = u16::MIN.wrapping_sub(4);
const TIMER_400_MICROSECONDS: u16 = u16::MIN.wrapping_sub(7);

#[derive(Debug)]
pub(in crate::driver) enum Request {
    Packet(Packet),
    WaitForIdle { frame: u8 },
    Idle { frame: u8 },
}

impl Request {
    pub(in crate::driver) fn new_packet(
        timer: Timer,
        transfer_length: TransferLength,
        source: Source,
    ) -> Self {
        schedule_timer(timer, transfer_length);
        Self::Packet(Packet::new(transfer_length, source))
    }

    pub(in crate::driver) fn new_wait_for_idle() -> Self {
        Self::WaitForIdle { frame: 0 }
    }

    pub(in crate::driver) fn new_idle(timer: Timer, transfer_length: TransferLength) -> Self {
        schedule_timer(timer, transfer_length);
        Self::Idle { frame: 0 }
    }

    pub(in crate::driver) fn vblank(
        &mut self,
        transfer_length: TransferLength,
    ) -> Result<(), Timeout> {
        match self {
            Self::Packet(packet) => {
                packet.timeout().map_err(Timeout::Packet)?;

                // The first byte when receiving a packet is triggered on vblank, not timer.
                if let Packet::Receive8 {
                    step: packet::receive::Step8::MagicByte1 { frame, .. },
                    ..
                }
                | Packet::Receive32 {
                    step: packet::receive::Step32::MagicByte { frame, .. },
                    ..
                } = packet
                    && *frame % frames::ONE_HUNDRED_MILLISECONDS as u16 == 0
                {
                    packet.push();
                    schedule_serial(transfer_length);
                }

                Ok(())
            }
            Self::WaitForIdle { frame } => {
                if *frame % frames::ONE_HUNDRED_MILLISECONDS == 0 {
                    // Send a new idle byte.
                    match transfer_length {
                        TransferLength::_8Bit => unsafe { SIODATA8.write_volatile(0x4b) },
                        TransferLength::_32Bit => unsafe {
                            SIODATA32.write_volatile(0x4b_4b_4b_4b);
                        },
                    }
                    schedule_serial(transfer_length);
                }
                if *frame > frames::THREE_SECONDS {
                    Err(Timeout::WaitForIdle)
                } else {
                    *frame += 1;
                    Ok(())
                }
            }
            Self::Idle { frame } => {
                if *frame > frames::THREE_SECONDS {
                    Err(Timeout::Idle)
                } else {
                    *frame += 1;
                    Ok(())
                }
            }
        }
    }

    pub(in crate::driver) fn timer(&mut self, transfer_length: TransferLength) {
        match self {
            Self::Packet(packet) => {
                packet.push();
                schedule_serial(transfer_length);
            }
            Self::WaitForIdle { .. } => {}
            Self::Idle { .. } => {
                match transfer_length {
                    TransferLength::_8Bit => unsafe { SIODATA8.write_volatile(0x4b) },
                    TransferLength::_32Bit => unsafe {
                        SIODATA32.write_volatile(0x4b_4b_4b_4b);
                    },
                }
                schedule_serial(transfer_length);
            }
        }
    }

    pub(in crate::driver) fn serial(
        self,
        adapter: &mut Adapter,
        transfer_length: &mut TransferLength,
        timer: Timer,
    ) -> Result<Option<Self>, Either<Error, command::Error>> {
        match self {
            Self::Packet(packet) => packet
                .pull(adapter, transfer_length)
                .map(|next_packet| {
                    next_packet.map(|packet| {
                        if !matches!(
                            packet,
                            Packet::Receive8 {
                                step: packet::receive::Step8::MagicByte1 { .. },
                                ..
                            } | Packet::Receive32 {
                                step: packet::receive::Step32::MagicByte { .. },
                                ..
                            }
                        ) {
                            // Only trigger the timer if this is not the start of a received packet.
                            // The first byte of the received packet is triggered on vblank instead.
                            schedule_timer(timer, *transfer_length);
                        }
                        Self::Packet(packet)
                    })
                })
                .map_err(|either| either.map_left(Error::Packet)),
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
            Self::Idle { .. } => match transfer_length {
                TransferLength::_8Bit => {
                    let byte = unsafe { SIODATA8.read_volatile() };
                    if byte == 0xd2 {
                        Ok(None)
                    } else {
                        Err(Either::Left(Error::NotIdle8(byte)))
                    }
                }
                TransferLength::_32Bit => {
                    let bytes = unsafe { SIODATA32.read_volatile() };
                    if bytes == 0xd2_d2_d2_d2 {
                        Ok(None)
                    } else {
                        Err(Either::Left(Error::NotIdle32(bytes)))
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

fn schedule_timer(timer: Timer, transfer_length: TransferLength) {
    let value = match transfer_length {
        TransferLength::_8Bit => TIMER_200_MICROSECONDS,
        TransferLength::_32Bit => TIMER_400_MICROSECONDS,
    };
    let control = timer::Control::new()
        .frequency(timer::Frequency::_1024)
        .interrupts(true)
        .start(true);
    unsafe {
        match timer {
            Timer::_0 => {
                TM0VAL.write_volatile(value);
                TM0CNT.write_volatile(control);
            }
            Timer::_1 => {
                TM1VAL.write_volatile(value);
                TM1CNT.write_volatile(control);
            }
            Timer::_2 => {
                TM2VAL.write_volatile(value);
                TM2CNT.write_volatile(control);
            }
            Timer::_3 => {
                TM3VAL.write_volatile(value);
                TM3CNT.write_volatile(control);
            }
        }
    }
}
