use super::{
    super::{Data, Timeout, communication, schedule_serial},
    Receive, ReceiveError,
};
use crate::{
    driver::frames,
    mmio::serial::{SIODATA8, TransferLength},
};
use either::Either;

#[derive(Debug)]
pub(in crate::driver::active) struct WaitForReceive {
    packet_frame: u16,
    serial_frame: u8,
    attempt: u8,
    communication_state: communication::State,
}

impl WaitForReceive {
    pub(super) fn new(attempt: u8) -> Self {
        Self {
            packet_frame: 0,
            serial_frame: 0,
            attempt,
            communication_state: communication::State::Send,
        }
    }

    fn reset(self) -> Self {
        Self {
            packet_frame: self.packet_frame,
            serial_frame: 0,
            attempt: self.attempt,
            communication_state: communication::State::Send,
        }
    }
}

impl super::super::WaitForReceive for WaitForReceive {
    type Receive = Receive;
    type ReceiveError = ReceiveError;

    fn vblank(&mut self) -> Result<(), Timeout> {
        if self.packet_frame > frames::FIFTEEN_SECONDS {
            Err(Timeout::Packet)
        } else if self.serial_frame > frames::THREE_SECONDS {
            Err(Timeout::Serial)
        } else {
            if self.packet_frame % frames::ONE_HUNDRED_MILLISECONDS as u16 == 0
                && matches!(self.communication_state, communication::State::Send)
            {
                // Send a new idle byte every 100 milliseconds.
                unsafe { SIODATA8.write_volatile(0x4b) };
                self.communication_state = communication::State::Receive;
                schedule_serial(TransferLength::_8Bit);
            }
            self.packet_frame += 1;
            self.serial_frame += 1;
            Ok(())
        }
    }

    fn serial(self, data: &mut Data) -> Result<Either<Self, Self::Receive>, Self::ReceiveError> {
        match self.communication_state {
            communication::State::Send => Ok(Either::Left(self)),
            communication::State::Receive => {
                let byte = unsafe { SIODATA8.read_volatile() };

                match byte {
                    0x99 => {
                        // Begin receiving the new packet.
                        *data = Data::new();
                        Ok(Either::Right(Receive::new(self.attempt)))
                    }
                    // Anything else should be ignored.
                    _ => Ok(Either::Left(self.reset())),
                }
            }
        }
    }
}
