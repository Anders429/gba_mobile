mod error;
mod timeout;

pub(in crate::driver) use error::Error;
pub(in crate::driver) use timeout::Timeout;

use super::{schedule_serial, schedule_timer};
use crate::{
    Timer,
    driver::frames,
    mmio::serial::{SIODATA8, SIODATA32, TransferLength},
};

#[derive(Debug)]
pub(in crate::driver::active) struct Idle {
    transfer_length: TransferLength,
    frame: u8,
}

impl Idle {
    pub(in crate::driver::active::flow) fn new(transfer_length: TransferLength) -> Self {
        Self {
            transfer_length,
            frame: 0,
        }
    }

    pub(in crate::driver::active::flow) fn vblank(mut self) -> Result<Self, Timeout> {
        if self.frame > frames::THREE_SECONDS {
            Err(Timeout)
        } else {
            self.frame += 1;
            Ok(self)
        }
    }

    pub(in crate::driver::active::flow) fn timer(&self) {
        match self.transfer_length {
            TransferLength::_8Bit => unsafe { SIODATA8.write_volatile(0x4b) },
            TransferLength::_32Bit => unsafe {
                SIODATA32.write_volatile(0x4b_4b_4b_4b);
            },
        }
        schedule_serial(self.transfer_length);
    }

    pub(in crate::driver::active::flow) fn serial(self) -> Result<(), Error> {
        match self.transfer_length {
            TransferLength::_8Bit => {
                let byte = unsafe { SIODATA8.read_volatile() };
                if byte == 0xd2 {
                    Ok(())
                } else {
                    Err(Error::Sio8(byte))
                }
            }
            TransferLength::_32Bit => {
                let bytes = unsafe { SIODATA32.read_volatile() };
                if bytes == 0xd2_d2_d2_d2 {
                    Ok(())
                } else {
                    Err(Error::Sio32(bytes))
                }
            }
        }
    }

    pub(in crate::driver::active::flow) fn schedule_timer(&self, timer: Timer) {
        schedule_timer(timer, self.transfer_length);
    }
}
