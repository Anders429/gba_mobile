use super::{
    super::{
        MAX_RETRIES, Payload, Timeout, communication, error, payload::Send as _, schedule_serial,
    },
    WaitForReceive,
};
use crate::{
    driver::{Command, frames},
    mmio::serial::{SIODATA8, TransferLength},
};
use either::Either;

#[derive(Debug)]
enum Step {
    MagicByte1,
    MagicByte2,

    HeaderCommand,
    HeaderEmptyByte,
    HeaderLength1,
    HeaderLength2,

    Data { index: u8 },

    Checksum1,
    Checksum2,

    FooterDevice,
    FooterCommand,
}

#[derive(Debug)]
pub(in crate::driver::active) struct Send<Payload>
where
    Payload: self::Payload,
{
    payload: Payload::Send,
    step: Step,
    checksum: u16,
    attempt: u8,
    frame: u8,
    communication_state: communication::State,
}

impl<Payload> Send<Payload>
where
    Payload: self::Payload,
{
    pub(in super::super) fn new(payload: Payload::Send) -> Self {
        Self {
            payload,
            step: Step::MagicByte1,
            checksum: 0,
            attempt: 0,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }

    /// Advance to the given step.
    fn next(self, step: Step) -> Self {
        Self {
            payload: self.payload,
            step,
            checksum: self.checksum,
            attempt: self.attempt,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }

    /// Re-attempt this packet.
    fn retry(self, new_attempt: u8) -> Self {
        Self {
            payload: self.payload,
            step: Step::MagicByte1,
            checksum: 0,
            attempt: new_attempt,
            frame: 0,
            communication_state: communication::State::Send,
        }
    }
}

impl<Payload> super::super::Send for Send<Payload>
where
    Payload: self::Payload,
{
    type WaitForReceive = WaitForReceive<Payload>;

    fn vblank(&mut self) -> Result<(), Timeout> {
        if self.frame > frames::THREE_SECONDS {
            return Err(Timeout::Serial);
        } else {
            self.frame += 1;
            Ok(())
        }
    }

    fn timer(&mut self) {
        if matches!(self.communication_state, communication::State::Send) {
            let byte = match self.step {
                Step::MagicByte1 => 0x99,
                Step::MagicByte2 => 0x66,
                Step::HeaderCommand => {
                    let byte = self.payload.command() as u8;
                    self.checksum = self.checksum.wrapping_add(byte as u16);
                    byte
                }
                Step::HeaderEmptyByte => 0x00,
                Step::HeaderLength1 => 0x00,
                Step::HeaderLength2 => {
                    let byte = self.payload.length();
                    self.checksum = self.checksum.wrapping_add(byte as u16);
                    byte
                }
                Step::Data { index } => {
                    let byte = self.payload.get(index);
                    self.checksum = self.checksum.wrapping_add(byte as u16);
                    byte
                }
                Step::Checksum1 => (self.checksum >> 8) as u8,
                Step::Checksum2 => self.checksum as u8,
                Step::FooterDevice => 0x81,
                Step::FooterCommand => 0x00,
            };

            self.communication_state = communication::State::Receive;
            unsafe { SIODATA8.write_volatile(byte) };
            schedule_serial(TransferLength::_8Bit);
        }
    }

    fn serial(self) -> Result<Either<Self, Self::WaitForReceive>, error::Send> {
        match self.communication_state {
            communication::State::Send => Ok(Either::Left(self)),
            communication::State::Receive => {
                let byte = unsafe { SIODATA8.read_volatile() };
                match self.step {
                    Step::MagicByte1 => Ok(Either::Left(self.next(Step::MagicByte2))),
                    Step::MagicByte2 => Ok(Either::Left(self.next(Step::HeaderCommand))),
                    Step::HeaderCommand => Ok(Either::Left(self.next(Step::HeaderEmptyByte))),
                    Step::HeaderEmptyByte => Ok(Either::Left(self.next(Step::HeaderLength1))),
                    Step::HeaderLength1 => Ok(Either::Left(self.next(Step::HeaderLength2))),
                    Step::HeaderLength2 => {
                        if self.payload.length() > 0 {
                            Ok(Either::Left(self.next(Step::Data { index: 0 })))
                        } else {
                            Ok(Either::Left(self.next(Step::Checksum1)))
                        }
                    }
                    Step::Data { index } => {
                        if let Some(next_index) = index.checked_add(1)
                            && self.payload.length() > next_index
                        {
                            Ok(Either::Left(self.next(Step::Data { index: next_index })))
                        } else {
                            Ok(Either::Left(self.next(Step::Checksum1)))
                        }
                    }
                    Step::Checksum1 => Ok(Either::Left(self.next(Step::Checksum2))),
                    Step::Checksum2 => Ok(Either::Left(self.next(Step::FooterDevice))),
                    Step::FooterDevice => Ok(Either::Left(self.next(Step::FooterCommand))),
                    Step::FooterCommand => {
                        let new_attempt = self.attempt + 1;
                        match Command::try_from(byte ^ 0x80) {
                            Ok(
                                Command::NotSupportedError
                                | Command::MalformedError
                                | Command::InternalError,
                            ) if new_attempt < MAX_RETRIES => {
                                Ok(Either::Left(self.retry(new_attempt)))
                            }
                            Ok(Command::NotSupportedError) => {
                                // Too many retries. Stop trying and set error state.
                                Err(error::Send::UnsupportedCommand(self.payload.command()))
                            }
                            Ok(Command::MalformedError) => {
                                // Too many retries. Stop trying and set error state.
                                Err(error::Send::Malformed)
                            }
                            Ok(Command::InternalError) => {
                                // Too many retries. Stop trying and set error state.
                                Err(error::Send::AdapterInternalError)
                            }
                            _ => {
                                // We don't verify anything here and simply assume the adapter
                                // responded with a correct command. If the adapter is in an invalid
                                // state, we will find out when receiving the response packet instead.
                                Ok(Either::Right(WaitForReceive::new(self.payload.finish(), 0)))
                            }
                        }
                    }
                }
            }
        }
    }
}
