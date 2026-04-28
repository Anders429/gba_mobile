use super::{
    super::{Data, Timeout, communication, error, schedule_serial},
    Receive, ReceiveError, receive_error,
};
use crate::{
    driver::{Command, frames},
    mmio::serial::{SIODATA32, TransferLength},
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
                // Send new idle bytes every 100 milliseconds.
                unsafe { SIODATA32.write_volatile(0x4b4b4b4b) };
                self.communication_state = communication::State::Receive;
                schedule_serial(TransferLength::_32Bit);
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
                let bytes = unsafe { SIODATA32.read_volatile().to_be_bytes() };

                match (bytes[0], bytes[1]) {
                    (0x99, 0x66) => {
                        let command_xor = bytes[2] & 0x80 == 0;
                        match Command::try_from(bytes[2] & 0x7f) {
                            Ok(command) => {
                                *data = Data::new();
                                data.command = command;
                                Ok(Either::Right(Receive::new(
                                    0,
                                    (bytes[2] as u16).wrapping_add(bytes[3] as u16),
                                    command_xor,
                                )))
                            }
                            Err(unknown) => Err(ReceiveError::new(
                                receive_error::Step::HeaderLength,
                                error::Receive::UnknownCommand(unknown),
                                self.attempt,
                            )),
                        }
                    }
                    // Anything else should be ignored.
                    _ => Ok(Either::Left(self.reset())),
                }
            }
        }
    }
}
