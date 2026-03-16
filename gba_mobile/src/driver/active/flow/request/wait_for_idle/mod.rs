mod timeout;

pub(in crate::driver) use timeout::Timeout;

use super::{communication, schedule_serial};
use crate::{
    driver::frames,
    mmio::serial::{SIODATA8, SIODATA32, TransferLength},
};

#[derive(Debug)]
pub(in crate::driver::active) struct WaitForIdle {
    transfer_length: TransferLength,
    frame: u8,
    communication_state: communication::State,
}

impl WaitForIdle {
    pub(in crate::driver::active::flow) fn new(transfer_length: TransferLength) -> Self {
        Self {
            transfer_length,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }

    pub(in crate::driver::active::flow) fn vblank(mut self) -> Result<Self, Timeout> {
        if self.frame % frames::ONE_HUNDRED_MILLISECONDS == 0 {
            if matches!(self.communication_state, communication::State::Send) {
                // Send a new idle byte.
                match self.transfer_length {
                    TransferLength::_8Bit => unsafe { SIODATA8.write_volatile(0x4b) },
                    TransferLength::_32Bit => unsafe {
                        SIODATA32.write_volatile(0x4b_4b_4b_4b);
                    },
                }
                self.communication_state = communication::State::Receive;
                schedule_serial(self.transfer_length);
            }
        }
        if self.frame > frames::THREE_SECONDS {
            Err(Timeout)
        } else {
            self.frame += 1;
            Ok(self)
        }
    }

    pub(in crate::driver::active::flow) fn serial(mut self) -> Option<Self> {
        match self.communication_state {
            communication::State::Send => Some(self),
            communication::State::Receive => match self.transfer_length {
                TransferLength::_8Bit => {
                    if unsafe { SIODATA8.read_volatile() } == 0xd2 {
                        None
                    } else {
                        self.communication_state = communication::State::Send;
                        Some(self)
                    }
                }
                TransferLength::_32Bit => {
                    if unsafe { SIODATA32.read_volatile() } == 0xd2_d2_d2_d2 {
                        None
                    } else {
                        self.communication_state = communication::State::Send;
                        Some(self)
                    }
                }
            },
        }
    }
}
